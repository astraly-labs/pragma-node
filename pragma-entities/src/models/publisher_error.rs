use axum::http::StatusCode;
use axum::Json;
use axum::response::IntoResponse;
use serde_json::json;
use utoipa::ToSchema;

#[derive(Debug, thiserror::Error, ToSchema)]
pub enum PublisherError {
    #[error("invalid key : {0}")]
    InvalidKey(String),
    #[error("invalid address : {0}")]
    InvalidAddress(String),
    #[error("inactive publisher : {0}")]
    InactivePublisher(String),
}

impl IntoResponse for PublisherError {
    fn into_response(self) -> axum::response::Response {
        let (status, err_msg) = match self {
            Self::InvalidKey(key) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Invalid Public Key {}", key),
            ),
            Self::InvalidAddress(address) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Invalid Address: {}", address),
            ),
            Self::InactivePublisher(publisher_name) => (
                StatusCode::FORBIDDEN,
                format!("Inactive Publisher: {}", publisher_name),
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