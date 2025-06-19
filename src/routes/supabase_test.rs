use axum::response::IntoResponse;
use axum::http::StatusCode;
use crate::supabase::test;

pub async fn supabase_route() -> impl IntoResponse {
    match test::test_supabase().await {
        Ok(body) => (StatusCode::OK, body),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", err)),
    }
}
