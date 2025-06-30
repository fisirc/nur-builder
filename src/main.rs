mod app_state;
mod github;
mod nur;
mod routes;
mod supabase;
mod utils;

use axum::routing::get;
use axum::{routing::post, Router};
use dotenvy::dotenv;
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
