use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use utoipa::ToSchema;

use crate::error::RedisError;

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
    #[error("could not deserialize the redis merkle tree into MerkleTree")]
    TreeDeserialization,
    #[error("could not generate a merkle proof for hash: {0}")]
    MerkleProof(String),
    #[error("no merkle feeds published for network: {0}")]
    NoBlocks(String),
}

impl From<RedisError> for MerkleFeedError {
    fn from(error: RedisError) -> Self {
        match error {
            RedisError::Connection => Self::RedisConnection,
            RedisError::OptionNotFound(block, name) => Self::OptionNotFound(block, name),
            RedisError::MerkleTreeNotFound(block) => Self::MerkleTreeNotFound(block),
            RedisError::InvalidOptionHash(r) => Self::InvalidOptionHash(r),
            RedisError::TreeDeserialization => Self::TreeDeserialization,
            RedisError::NoBlocks(network) => Self::NoBlocks(network),
            RedisError::InternalServerError => Self::InternalServerError,
        }
    }
}

impl IntoResponse for MerkleFeedError {
    fn into_response(self) -> axum::response::Response {
        let (status, err_msg) = match self {
            Self::InvalidOptionHash(hash) => (
                StatusCode::BAD_REQUEST,
                format!(
                    "Option hash is not a correct 0x prefixed hexadecimal hash: {hash}"
                ),
            ),
            Self::OptionNotFound(block_number, instrument_name) => (
                StatusCode::NOT_FOUND,
                format!(
                    "MerkleFeed option for instrument {instrument_name} has not been found for block {block_number}",
                ),
            ),
            Self::MerkleTreeNotFound(block_number) => (
                StatusCode::NOT_FOUND,
                format!("MerkleFeed tree not found for block {block_number}"),
            ),
            Self::RedisConnection => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Could not establish a connection with the Redis database".to_string(),
            ),
            Self::TreeDeserialization => (
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Internal server error: could not decode Redis merkle tree"),
            ),
            Self::NoBlocks(network) => (
                StatusCode::NOT_FOUND,
                format!("No merkle feeds published for network {network}"),
            ),
            Self::MerkleProof(hash) => (
                StatusCode::NOT_FOUND,
                format!("Could not generate a valid merkle proof for hash {hash}"),
            ),
            Self::InternalServerError => (
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
