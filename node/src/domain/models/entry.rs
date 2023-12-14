use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use starknet::core::crypto::EcdsaVerifyError;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::infra::errors::InfraError;

use super::publisher::PublisherError;

#[derive(Clone, Debug, PartialEq, ToSchema)]
pub struct EntryModel {
    pub id: Uuid,
    pub pair_id: String,
    pub publisher: String,
    pub source: String,
    pub timestamp: u64,
    pub price: u128,
}

#[derive(Debug, thiserror::Error, ToSchema)]
pub enum VolatilityError {
    #[error("invalid timestamps range: {0} > {1}")]
    InvalidTimestampsRange(u64, u64),
}

#[derive(Debug, thiserror::Error, ToSchema)]
pub enum EntryError {
    #[error("internal server error")]
    InternalServerError,
    #[error("entry not found: {0}")]
    NotFound(String),
    #[error("infra error: {0}")]
    InfraError(InfraError),
    #[error("invalid signature")]
    InvalidSignature(EcdsaVerifyError),
    #[error("unauthorized request")]
    Unauthorized,
    #[error("publisher error: {0}")]
    PublisherError(#[from] PublisherError),
    #[error("invalid input amount: {0}")]
    InvalidAmount(String),
    #[error("pair id invalid: {0}")]
    UnknownPairId(String),
    #[error("volatility error: {0}")]
    VolatilityError(#[from] VolatilityError),
    #[error("can't publish data: {0}")]
    PublishData(String),
    #[error("can't build publish message: {0}")]
    BuildPublish(String)
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
            Self::InvalidSignature(err) => (
                StatusCode::BAD_REQUEST,
                format!("Invalid signature: {}", err),
            ),
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "Unauthorized publisher".to_string(),
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
