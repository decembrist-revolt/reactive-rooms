pub mod dto;
pub mod handlers;
pub mod routes;

use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use serde_json::json;

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: &'static str,
}

pub fn error_response(status: StatusCode, message: &'static str) -> impl IntoResponse {
    (status, Json(ErrorResponse { error: message }))
}

pub async fn health() -> Json<serde_json::Value> {
    Json(json!({"status": "UP"}))
}

pub async fn not_found() -> impl IntoResponse {
    error_response(StatusCode::NOT_FOUND, "Not found")
}
