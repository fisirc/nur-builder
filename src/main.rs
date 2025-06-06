use axum::extract::Request;
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{
    extract::{Json, State},
    http::StatusCode,
    routing::post,
    Router,
};
use axum::body::to_bytes;
use axum::body::{Body, Bytes};
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
use tokio::process::Command;
use tokio::net::TcpListener;

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

// async fn handle_webhook(
//     headers: HeaderMap,
//     State(state): State<Arc<AppState>>,
//     Json(payload): Json<GitHubPushEvent>,
// ) -> StatusCode {
//     if let Some(sig) = headers.get("X-Hub-Signature-256") {
//         let sig_str = sig.to_str().unwrap_or("");
//         if !verify_signature(sig_str, &payload, &state.webhook_secret) {
//             println!("Invalid signature");
//             return StatusCode::UNAUTHORIZED;
//         }
//     }

//     println!("Push received to {}", payload.repository.full_name);

//     // Get installation access token
//     let jwt = create_jwt(&state.app_id, &state.encoding_key);

//     let token_res = state
//         .client
//         .post(format!(
//             "https://api.github.com/app/installations/{}/access_tokens",
//             payload.installation.id
//         ))
//         .bearer_auth(jwt)
//         .header("Accept", "application/vnd.github+json")
//         .header("User-Agent", "wasm-builder-app")
//         .send()
//         .await
//         .unwrap();

//     let token_json: serde_json::Value = token_res.json().await.unwrap();
//     let token = token_json["token"].as_str().unwrap();

//     // Clone and build
//     let clone_url = payload
//         .repository
//         .clone_url
//         .replace("https://", &format!("https://x-access-token:{}@", token));
//     let dir_name = payload.repository.full_name.split('/').last().unwrap();

//     let _ = Command::new("git")
//         .args(["clone", &clone_url, dir_name])
//         .output()
//         .await;

//     let _ = Command::new("cargo")
//         .args(["build", "--target", "wasm32-unknown-unknown"])
//         .current_dir(dir_name)
//         .output()
//         .await;

//     println!("Build completed for repo: {}", payload.repository.full_name);

//     StatusCode::OK
// }


async fn webhook_handler(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> StatusCode {
    let (parts, body) = req.into_parts();
    let body_bytes = to_bytes(body, usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body_bytes);

    // Verify signature
    if let Some(sig) = headers.get("X-Hub-Signature-256") {
        let sig_str = sig.to_str().unwrap_or("");
        if !verify_signature(sig_str, &body_bytes, &state.webhook_secret) {
            println!("❌ Invalid signature");
            return StatusCode::UNAUTHORIZED;
        }
    }

    // Print GitHub event
    if let Some(event_type) = headers.get("X-GitHub-Event") {
        println!("✅ Event: {}", event_type.to_str().unwrap_or("Unknown"));
    } else {
        println!("⚠️ No X-GitHub-Event header");
    }

    // Log payload
    println!("📦 Payload:\n{}", body_str);

    StatusCode::OK
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
