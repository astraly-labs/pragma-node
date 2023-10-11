use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use uuid::Uuid;

use crate::infra::errors::InfraError;

#[derive(Clone, Debug, PartialEq)]
pub struct EntryModel {
    pub id: Uuid,
    pub pair_id: String,
    pub publisher: String,
    pub timestamp: u64,
    pub price: u128,
}

#[derive(Debug)]
pub enum EntryError {
    InternalServerError,
    NotFound(String),
    InfraError(InfraError),
}

impl IntoResponse for EntryError {
    fn into_response(self) -> axum::response::Response {
        let (status, err_msg) = match self {
            Self::NotFound(pair_id) => (
                StatusCode::NOT_FOUND,
                format!("EntryModel with pair id {} has not been found", pair_id),
            ),
            Self::InfraError(db_error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal server error: {}", db_error),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Internal server error"),
            ),
        };
        (
            status,
            Json(
                json!({"resource":"EntryModel", "message": err_msg, "happened_at" : chrono::Utc::now() }),
            ),
        )
            .into_response()
    }
}
