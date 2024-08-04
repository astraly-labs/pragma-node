use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

use crate::error::InfraError;

#[derive(Debug, thiserror::Error)]
pub enum CheckpointError {
    #[error("internal server error")]
    InternalServerError,
    #[error("invalid limit : {0}")]
    InvalidLimit(u64),
    #[error("no checkpoints found for requested pair")]
    NotFound,
}

impl From<InfraError> for CheckpointError {
    fn from(error: InfraError) -> Self {
        match error {
            InfraError::InternalServerError => Self::InternalServerError,
            InfraError::NotFound => Self::NotFound,
            InfraError::RoutingError => Self::InternalServerError,
            InfraError::InvalidTimestamp(_) => Self::InternalServerError,
            InfraError::NonZeroU32Conversion(_) => Self::InternalServerError,
            InfraError::AxumError(_) => Self::InternalServerError,
            InfraError::RedisError(_) => Self::InternalServerError,
        }
    }
}

impl IntoResponse for CheckpointError {
    fn into_response(self) -> axum::response::Response {
        let (status, err_msg) = match self {
            Self::InvalidLimit(limit) => {
                (StatusCode::BAD_REQUEST, format!("Invalid Limit {}", limit))
            }
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                String::from("No checkpoints found for requested pair"),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Internal server error"),
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
