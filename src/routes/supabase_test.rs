use crate::supabase::crud;
use axum::http::StatusCode;
use axum::response::IntoResponse;

pub async fn supabase_route() -> impl IntoResponse {
    match crud::test_supabase().await {
        Ok(body) => (StatusCode::OK, body),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", err)),
    }
}
