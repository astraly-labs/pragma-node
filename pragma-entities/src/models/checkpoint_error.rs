use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;

use crate::error::InfraError;

#[derive(Debug, thiserror::Error)]
pub enum CheckpointError {
    // 400
    #[error("invalid limit : {0}")]
    InvalidLimit(u64),
    // 404
    #[error("no checkpoints found for pair {0}")]
    CheckpointNotFound(String),
    // 500
    #[error("internal server error{0}")]
    InternalServerError(String),
}

impl From<InfraError> for CheckpointError {
    fn from(error: InfraError) -> Self {
        match error {
            // 404
            InfraError::CheckpointNotFound(pair_id) => Self::CheckpointNotFound(pair_id),
            // 500
            InfraError::NoRpcAvailable(network) => {
                Self::InternalServerError(format!(": no RPC available for network {network}"))
            }
            // Those errors should never proc for the Checkpoints.
            _ => Self::InternalServerError(String::default()),
        }
    }
}

impl IntoResponse for CheckpointError {
    fn into_response(self) -> axum::response::Response {
        let (status, err_msg) = match self {
            Self::InvalidLimit(limit) => {
                (StatusCode::BAD_REQUEST, format!("Invalid Limit {limit}"))
            }
            Self::CheckpointNotFound(pair_id) => (
                StatusCode::NOT_FOUND,
                format!("No checkpoints found for pair {pair_id}"),
            ),
            Self::InternalServerError(details) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal server error{details}"),
            ),
        };
        (
            status,
            Json(
                json!({"resource":"Checkpoint", "message": err_msg, "happened_at" : chrono::Utc::now() }),
            ),
        )
            .into_response()
    }
}
