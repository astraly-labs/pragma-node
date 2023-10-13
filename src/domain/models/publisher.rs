use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use uuid::Uuid;

use crate::infra::errors::InfraError;

#[derive(Clone, Debug, PartialEq)]
pub struct PublisherModel {
    pub id: Uuid,
    pub name: String,
    pub master_key: String,
    pub active_key: String,
    pub active: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum PublisherError {
    #[error("internal server error")]
    InternalServerError,
    #[error("publisher not found: {0}")]
    NotFound(String),
    #[error("infra error: {0}")]
    InfraError(InfraError),
    #[error("invalid key : {0}")]
    InvalidKey(String),
}

impl IntoResponse for PublisherError {
    fn into_response(self) -> axum::response::Response {
        let (status, err_msg) = match self {
            Self::NotFound(name) => (
                StatusCode::NOT_FOUND,
                format!("PublisherModel with name {} has not been found", name),
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
                json!({"resource":"PublisherModel", "message": err_msg, "happened_at" : chrono::Utc::now() }),
            ),
        )
            .into_response()
    }
}
