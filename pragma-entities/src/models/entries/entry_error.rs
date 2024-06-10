use crate::error::InfraError;
use crate::models::publisher_error::PublisherError;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use starknet::core::crypto::{EcdsaSignError, EcdsaVerifyError};
use utoipa::ToSchema;

#[derive(Debug, thiserror::Error, ToSchema)]
pub enum VolatilityError {
    #[error("invalid timestamps range: {0} > {1}")]
    InvalidTimestampsRange(u64, u64),
}

#[derive(Debug, thiserror::Error, ToSchema)]
pub enum EntryError {
    #[error("internal server error")]
    InternalServerError,
    #[error("bad request")]
    BadRequest,
    #[error("entry not found: {0}")]
    NotFound(String),
    #[error("infra error: {0}")]
    InfraError(InfraError),
    #[error("invalid signature")]
    InvalidSignature(EcdsaVerifyError),
    #[error("could not sign price")]
    InvalidSigner(EcdsaSignError),
    #[error("unauthorized request")]
    Unauthorized,
    #[error("invalid timestamp")]
    InvalidTimestamp,
    #[error("publisher error: {0}")]
    PublisherError(#[from] PublisherError),
    #[error("pair id invalid: {0}")]
    UnknownPairId(String),
    #[error("volatility error: {0}")]
    VolatilityError(#[from] VolatilityError),
    #[error("can't publish data: {0}")]
    PublishData(String),
    #[error("can't build publish message: {0}")]
    BuildPublish(String),
}

impl From<InfraError> for EntryError {
    fn from(error: InfraError) -> Self {
        match error {
            InfraError::InternalServerError => Self::InternalServerError,
            InfraError::NotFound => Self::NotFound("Unknown".to_string()),
            InfraError::InvalidTimeStamp => Self::InternalServerError,
        }
    }
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
            Self::InvalidTimestamp => (StatusCode::BAD_REQUEST, "Invalid timestamp".to_string()),
            Self::PublisherError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Publisher error: {}", err),
            ),
            Self::BadRequest => (StatusCode::BAD_REQUEST, "Bad request".to_string()),
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
