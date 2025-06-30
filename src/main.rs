mod app_state;
mod github;
mod nur;
mod routes;
mod supabase;
mod utils;

use axum::routing::get;
use axum::{routing::post, Router};
use dotenvy::dotenv;
use tokio::process::Command;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;

use crate::app_state::build_app_state;
use crate::routes::supabase_test::supabase_route;
use crate::routes::webhook_handler::webhook_handler;

#[tokio::main]
async fn main() {
    dotenv().ok();

    // console_subscriber::init();

    // tracing_subscriber::fmt()
    //     .with_env_filter("debug,tokio=trace")
    //     .init();

    tokio::spawn(let_the_shit_fail());

    let app_state = build_app_state().expect("Failed to build AppState");

    let app = Router::new()
        .route("/webhook", post(webhook_handler))
        .route("/supabase-test", get(supabase_route))
        .route("/", get(|| async { "Hola Nur!!!" }))
        .with_state(Arc::new(app_state));

    println!("Listening on http://0.0.0.0:3000");

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

/// It seems that it's inevitable that the first podman run
/// will always fail. The linked issue is not resolved yet, so
/// until that we just trigger the first podamn error.
/// See <https://github.com/containers/podman/issues/24737>
async fn let_the_shit_fail() {
    let status = match Command::new("podman")
        .args([
            "run",
            "--rm",
            "-w",
            "/tmp",
            "ghcr.io/fisirc/rust-builder:latest",
            "sh",
            "-c",
            "true",
        ])
        .status()
        .await {
            Ok(r) => r,
            Err(_) => return,
        };

    if !status.success() {
        println!("Trigerring first podman fail successfully!");
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("Signal received, starting graceful shutdown");
}
