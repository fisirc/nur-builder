use axum::extract::Request;
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{
    extract::{State},
    http::StatusCode,
    routing::post,
    Router,
};
use axum::body::to_bytes;
use axum::body::{Body};
use dotenvy::dotenv;
use hmac::{Hmac, Mac};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::env;
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::net::TcpListener;
use tokio::process::Command;

#[derive(Deserialize, Serialize, Debug)]
struct GitHubPushEvent {
    repository: Repository,
    installation: Installation,
}

#[derive(Deserialize, Serialize, Debug)]
struct Repository {
    full_name: String,
    clone_url: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct Installation {
    id: u64,
}

#[derive(Serialize)]
struct Claims {
    iat: usize,
    exp: usize,
    iss: String,
}

struct AppState {
    client: Client,
    encoding_key: EncodingKey,
    app_id: String,
    webhook_secret: String,
}

// Nur files
#[derive(Debug, Deserialize)]
struct NurBuild {
    command: String,
    output: String,
}

#[derive(Debug, Deserialize)]
struct NurConfig {
    name: String,
    language: String,
    build: NurBuild,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let app_id = env::var("APP_ID").expect("APP_ID not set");
    let webhook_secret = env::var("WEBHOOK_SECRET").expect("WEBHOOK_SECRET not set");
    let private_key_path = env::var("PRIVATE_KEY_PATH").expect("PRIVATE_KEY_PATH not set");

    let private_key =
        std::fs::read_to_string(&private_key_path).expect("Failed to read private key");

    let app_state = AppState {
        client: Client::new(),
        encoding_key: EncodingKey::from_rsa_pem(private_key.as_bytes()).unwrap(),
        app_id,
        webhook_secret,
    };

    let app = Router::new()
        .route("/webhook", post(webhook_handler))
        .route("/", get(|| async { "Hola Nur!!!"} ))
        .with_state(Arc::new(app_state));

    println!("Listening on http://0.0.0.0:3000");

    // ✅ Forma moderna con TcpListener
    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn webhook_handler(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> StatusCode {
    let (parts, body) = req.into_parts();
    let body_bytes = to_bytes(body, usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body_bytes);

    // ✅ 1. Verificar firma
    if let Some(sig) = headers.get("X-Hub-Signature-256") {
        let sig_str = sig.to_str().unwrap_or("");
        if !verify_signature(sig_str, &body_bytes, &state.webhook_secret) {
            println!("❌ Invalid signature");
            return StatusCode::UNAUTHORIZED;
        }
    }

    // ✅ 2. Parsear evento
    let event: GitHubPushEvent = match serde_json::from_slice(&body_bytes) {
        Ok(e) => e,
        Err(e) => {
            println!("❌ Invalid JSON payload: {:?}", e);
            return StatusCode::BAD_REQUEST;
        }
    };

    println!("✅ Push event: {:?}", event.repository.full_name);

    // ✅ 3. Crear JWT
    let jwt = create_jwt(&state.app_id, &state.encoding_key);

    // ✅ 4. Obtener token de instalación
    let token_res = state
        .client
        .post(format!(
            "https://api.github.com/app/installations/{}/access_tokens",
            event.installation.id
        ))
        .bearer_auth(jwt)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "nur-wasm-builder")
        .send()
        .await
        .unwrap();

    let token_json: serde_json::Value = token_res.json().await.unwrap();
    let token = token_json["token"].as_str().unwrap();

    // ✅ 5. Clonar el repo
    let clone_url = event
        .repository
        .clone_url
        .replace("https://", &format!("https://x-access-token:{}@", token));
    let repo_name = event.repository.full_name.split('/').last().unwrap();

    println!("📥 Cloning {}...", clone_url);

    // Clone the repo (shallow)
    let clone_output = Command::new("git")
        .args(["clone", "--depth=1", &clone_url, repo_name])
        .output()
        .await;

    if let Err(e) = clone_output {
        println!("❌ Error cloning repo: {:?}", e);
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    // Get latest commit hash and message
    let log_output = Command::new("git")
        .args(["log", "-1", "--pretty=format:%H%n%s"])
        .current_dir(repo_name)
        .output()
        .await;

    match log_output {
        Ok(output) => {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let mut lines = output_str.lines();
            let commit_hash = lines.next().unwrap_or("unknown");
            let commit_msg = lines.next().unwrap_or("no commit message");

            println!("🔐 Cloned commit hash: {}", commit_hash);
            println!("📝 Commit message: {}", commit_msg);
        }
        Err(e) => {
            println!("⚠️ Failed to get commit info: {:?}", e);
        }
    }


    // ✅ 6. Ejecutar build
    match run_nur_build(repo_name).await {
        Ok(_) => {
            println!("✅ Build completed successfully.");
            StatusCode::OK
        }
        Err(e) => {
            println!("❌ Build error: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}


fn create_jwt(app_id: &str, key: &EncodingKey) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    let claims = Claims {
        iat: now,
        exp: now + 600,
        iss: app_id.to_string(),
    };
    encode(&Header::new(Algorithm::RS256), &claims, key).unwrap()
}

fn verify_signature(signature: &str, body: &[u8], secret: &str) -> bool {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body);
    let expected = format!("sha256={:x}", mac.finalize().into_bytes());
    signature == expected
}

async fn run_nur_build(dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = format!("{}/nurfile.yaml", dir);
    let contents = std::fs::read_to_string(&config_path)?;
    let config: NurConfig = serde_yaml::from_str(&contents)?;

    println!("📦 Building {}...", config.name);
    println!("📄 Raw build command from nurfile: {}", config.build.command);
    println!("📄 Expected output path from nurfile: {}", config.build.output);

    // Detectar si es Rust + WASM
    let is_rust_wasm = config.language.to_lowercase() == "rust" && config.build.output.ends_with(".wasm");

    // Forzar comando correcto para Rust WASM
    let (command, args): (String, Vec<&str>) = if is_rust_wasm {
        println!("⚙️  Rust WASM project detected. Overriding build command with cargo wasm32-wasip1 build.");
        (
            "cargo".to_string(),
            vec!["build", "--target", "wasm32-wasip1", "--release"],
        )
    } else {
        let mut parts = config.build.command.split_whitespace();
        let cmd = parts.next().unwrap_or("sh").to_string();
        (cmd, parts.collect())
    };

    println!("🚀 Running: {} {:?}", command, args);

    let output = Command::new(&command)
        .args(&args)
        .current_dir(dir)
        .output()
        .await?;

    if !output.status.success() {
        println!("❌ Build failed:\n{}", String::from_utf8_lossy(&output.stderr));
        return Err("Build failed".into());
    }

    // Validar que el output especificado exista
    let output_path = format!("{}/{}", dir, config.build.output);
    println!("🔍 Checking if build output exists at: {}", output_path);

    if !std::path::Path::new(&output_path).exists() {
        // Sugerencia inteligente para Rust/WASM
        if is_rust_wasm {
            let suggested_name = config.name.replace("-", "_"); // Coincide con nombre de crate generado por Rust
            let suggested_path = format!("{}/target/wasm32-wasip1/release/{}.wasm", dir, suggested_name);
            println!("💡 Hint: Common Rust WASM output is `{}`", suggested_path);
        }

        return Err(format!("❌ Build output file not found at: {}", output_path).into());
    }

    println!("✅ Build output found at: {}", output_path);
    Ok(())
}

