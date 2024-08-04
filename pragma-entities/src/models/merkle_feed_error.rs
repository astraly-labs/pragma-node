use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use utoipa::ToSchema;

use crate::InfraError;

#[derive(Debug, thiserror::Error, ToSchema)]
pub enum MerkleFeedError {
    #[error("internal server error")]
    InternalServerError,
    #[error("could not establish a connection with Redis")]
    RedisConnection,
    #[error("option for instrument {1} not found for block {0}")]
    OptionNotFound(u64, String),
    #[error("merkle tree not found for block {0}")]
    MerkleTreeNotFound(u64),
    #[error("invalid option hash, could not convert to felt: {0}")]
    InvalidOptionHash(String),
}

impl From<InfraError> for MerkleFeedError {
    fn from(error: InfraError) -> Self {
        match error {
            InfraError::InternalServerError => Self::InternalServerError,
            InfraError::NotFound => Self::InternalServerError,
            InfraError::RoutingError => Self::InternalServerError,
            InfraError::InvalidTimestamp(_) => Self::InternalServerError,
            InfraError::NonZeroU32Conversion(_) => Self::InternalServerError,
            InfraError::AxumError(_) => Self::InternalServerError,
            InfraError::RedisError(_) => Self::InternalServerError,
        }
    }
}

impl IntoResponse for MerkleFeedError {
    fn into_response(self) -> axum::response::Response {
        let (status, err_msg) = match self {
            Self::OptionNotFound(block_number, instrument_name) => (
                StatusCode::NOT_FOUND,
                format!(
                    "MerkleFeed option for instrument {} has not been found for block {}",
                    instrument_name, block_number
                ),
            ),
            Self::InvalidOptionHash(hash) => (
                StatusCode::BAD_REQUEST,
                format!(
                    "Option hash is not a correct 0x prefixed hexadecimal hash: {}",
                    hash
                ),
            ),
            Self::MerkleTreeNotFound(block_number) => (
                StatusCode::NOT_FOUND,
                format!("MerkleFeed tree not found for block {}", block_number),
            ),
            Self::RedisConnection => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Could not establish a connection with the Redis database".to_string(),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Internal server error"),
            ),
        };
        (
            status,
            Json(
                json!({"resource":"MerkleFeed", "message": err_msg, "happened_at" : chrono::Utc::now() }),
            ),
        )
            .into_response()
    }
}
