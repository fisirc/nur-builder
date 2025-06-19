mod github;
mod nur;
mod app_state;
mod utils;
mod routes;
mod supabase;

use axum::routing::get;
use axum::{
    routing::post,
    Router,
};
use dotenvy::dotenv;
use std::{
    sync::Arc,
};
use tokio::net::TcpListener;

use crate::app_state::{build_app_state};
use crate::routes::supabase_test::supabase_route;
use crate::routes::webhook_handler::webhook_handler;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let app_state = build_app_state().expect("Failed to build AppState");

    let app = Router::new()
        .route("/webhook", post(webhook_handler))
        .route("/supabase-test", get(supabase_route))
        .route("/", get(|| async { "Hola Nur!!!"} ))
        .with_state(Arc::new(app_state));

    println!("Listening on http://0.0.0.0:3000");

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}