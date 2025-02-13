use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use utoipa::ToSchema;

use crate::error::InfraError;

#[derive(Debug, thiserror::Error, ToSchema)]
pub enum PublisherError {
    #[error("internal server error")]
    InternalServerError,
    #[error("invalid key : {0}")]
    InvalidKey(String),
    #[error("invalid address : {0}")]
    InvalidAddress(String),
    #[error("inactive publisher : {0}")]
    InactivePublisher(String),
    #[error("no publishers found")]
    NotFound,
}

impl From<InfraError> for PublisherError {
    fn from(error: InfraError) -> Self {
        match error {
            InfraError::NotFound => Self::NotFound,
            _ => Self::InternalServerError,
        }
    }
}

impl IntoResponse for PublisherError {
    fn into_response(self) -> axum::response::Response {
        let (status, err_msg) = match self {
            Self::InvalidKey(key) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Invalid Public Key {key}"),
            ),
            Self::InvalidAddress(address) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Invalid Address: {address}"),
            ),
            Self::InactivePublisher(publisher_name) => (
                StatusCode::FORBIDDEN,
                format!("Inactive Publisher: {publisher_name}"),
            ),
            Self::NotFound => (StatusCode::NOT_FOUND, "No publishers found".to_string()),
            Self::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error".to_string(),
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
