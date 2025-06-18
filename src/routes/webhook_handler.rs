use crate::app_state::{AppState};
use crate::github::jwt::create_jwt;
use crate::github::models::GitHubPushEvent;
use crate::nur::build::run_nur_build;
use crate::utils::verify_signature;

use axum::extract::Request;
use axum::http::HeaderMap;
use axum::{
    extract::{State},
    http::StatusCode,
};
use axum::body::to_bytes;
use axum::body::{Body};
use std::{
    sync::Arc,
};

pub async fn webhook_handler(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> StatusCode {
    let (_parts, body) = req.into_parts();
    let body_bytes = to_bytes(body, usize::MAX).await.unwrap();
    let _body_str = String::from_utf8_lossy(&body_bytes);

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

    // ✅ 5. URL para clonar la repo
    let clone_url = event
        .repository
        .clone_url
        .replace("https://", &format!("https://x-access-token:{}@", token));

    // ✅ 6. Ejecutar build
    match run_nur_build(&clone_url).await {
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