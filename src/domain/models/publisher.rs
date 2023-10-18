use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, ToSchema)]
pub struct PublisherModel {
    pub id: Uuid,
    pub name: String,
    pub master_key: String,
    pub active_key: String,
    pub account_address: String,
    pub active: bool,
}

#[derive(Debug, thiserror::Error, ToSchema)]
pub enum PublisherError {
    #[error("invalid key : {0}")]
    InvalidKey(String),
    #[error("invalid address : {0}")]
    InvalidAddress(String),
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
