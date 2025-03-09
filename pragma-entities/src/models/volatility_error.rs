use axum::response::IntoResponse;
use pragma_common::{
    timestamp::TimestampError,
    types::{AggregationMode, Interval},
};
use utoipa::ToSchema;

use crate::InfraError;

#[derive(Debug, thiserror::Error, ToSchema)]
pub enum VolatilityError {
    // 400
    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(#[from] TimestampError),
    #[error("unsupported interval {0:?} for aggregation {1:?}")]
    InvalidInterval(Interval, AggregationMode),
    // 404
    #[error("no entries found for pair {0}")]
    EntryNotFound(String),
    // 500
    #[error("database error: {0}")]
    DbError(String),
    #[error("interval server error")]
    InternalServerError,
}

impl From<InfraError> for VolatilityError {
    fn from(error: InfraError) -> Self {
        match error {
            InfraError::InvalidTimestamp(err) => Self::InvalidTimestamp(err),
            InfraError::UnsupportedInterval(interval, mode) => {
                Self::InvalidInterval(interval, mode)
            }
            InfraError::DbInteractionError(e) => Self::DbError(e.to_string()),
            InfraError::DbResultError(e) => Self::DbError(e.to_string()),
            InfraError::DbPoolError(e) => Self::DbError(e.to_string()),
            // Those errors should never proc for Entry
            _ => Self::InternalServerError,
        }
    }
}

impl IntoResponse for VolatilityError {
    fn into_response(self) -> axum::response::Response {
        use axum::Json;
        use axum::http::StatusCode;
        use serde_json::json;

        let (status, err_msg) = match self {
            // 400 - Bad Request errors
            Self::InvalidTimestamp(err) => {
                (StatusCode::BAD_REQUEST, format!("Invalid timestamp: {err}"))
            }
            Self::InvalidInterval(interval, mode) => (
                StatusCode::BAD_REQUEST,
                format!("Unsupported interval {interval:?} for aggregation {mode:?}"),
            ),

            // 404 - Not Found errors
            Self::EntryNotFound(pair_id) => (
                StatusCode::NOT_FOUND,
                format!("No entries found for pair {pair_id}"),
            ),

            // 500 - Server errors
            Self::DbError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {err}"),
            ),
            Self::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };

        (
            status,
            Json(json!({
                "resource": "VolatilityModel",
                "message": err_msg,
                "happened_at": chrono::Utc::now()
            })),
        )
            .into_response()
    }
}
