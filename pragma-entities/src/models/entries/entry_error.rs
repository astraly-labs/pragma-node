use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use pragma_common::signing::SignerError;
use serde_json::json;
use utoipa::ToSchema;

use pragma_common::timestamp::TimestampError;
use pragma_common::types::{AggregationMode, Interval};

use crate::PublisherError;
use crate::error::{InfraError, WebSocketError};

#[derive(Debug, thiserror::Error, ToSchema)]
pub enum EntryError {
    // 400 Error - Bad Requests
    #[error("invalid signature: {0}")]
    InvalidSignature(#[from] SignerError),
    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(#[from] TimestampError),
    #[error("invalid expiry")]
    InvalidExpiry,
    #[error("unsupported interval {0:?} for aggregation {1:?}")]
    InvalidInterval(Interval, AggregationMode),
    #[error("invalid login message: {0}")]
    InvalidLoginMessage(String),

    // 401 Error - Unauthorized
    #[error("unauthorized request: {0}")]
    Unauthorized(String),

    // 404 errors
    #[error("pair not found: {0}")]
    PairNotFound(String),
    #[error("entry not found: {0}")]
    EntryNotFound(String),
    #[error("publisher not found: {0}")]
    PublisherNotFound(String),
    #[error("missing data for routing on pair: {0}")]
    RouteNotFound(String),
    #[error("history not found")]
    HistoryNotFound,

    // ??? publishing...
    #[error("can't publish data: {0}")]
    PublisherError(#[from] PublisherError),
    #[error("can't publish data: {0}")]
    PublishData(String),
    #[error("can't build publish message: {0}")]
    BuildPublish(String),

    // Internal shit
    #[error("could not sign price")]
    InvalidSigner,

    // 500 Error - Internal server error
    #[error("internal server error: {0}")]
    InternalServerError(String),

    #[error("websocket error: {0}")]
    WebSocketError(#[from] WebSocketError),
}

impl From<InfraError> for EntryError {
    fn from(error: InfraError) -> Self {
        match error {
            InfraError::InvalidTimestamp(err) => Self::InvalidTimestamp(err),
            InfraError::UnsupportedInterval(interval, mode) => {
                Self::InvalidInterval(interval, mode)
            }
            InfraError::RoutingError(pair_id) => Self::RouteNotFound(pair_id),
            InfraError::EntryNotFound(entry_id) => Self::EntryNotFound(entry_id),
            InfraError::PairNotFound(pair_id) => Self::PairNotFound(pair_id),
            // Those errors should never proc for Entry
            e => Self::InternalServerError(e.to_string()),
        }
    }
}

impl IntoResponse for EntryError {
    fn into_response(self) -> axum::response::Response {
        let (status, err_msg) = match self {
            Self::InvalidSignature(err) => {
                (StatusCode::BAD_REQUEST, format!("Invalid signature: {err}"))
            }
            Self::InvalidTimestamp(reason) => (
                StatusCode::BAD_REQUEST,
                format!("Invalid timestamp: {reason}"),
            ),
            Self::InvalidExpiry => (StatusCode::BAD_REQUEST, "Invalid expiry".to_string()),
            Self::InvalidInterval(interval, mode) => (
                StatusCode::BAD_REQUEST,
                format!("Unsupported interval {interval:?} for aggregation {mode:?}"),
            ),
            Self::InvalidLoginMessage(msg) => (
                StatusCode::BAD_REQUEST,
                format!("Invalid login message: {msg}"),
            ),
            Self::Unauthorized(reason) => (
                StatusCode::UNAUTHORIZED,
                format!("Unauthorized request: {reason}"),
            ),
            Self::PairNotFound(pair_id) => {
                (StatusCode::NOT_FOUND, format!("Pair not found: {pair_id}"))
            }
            Self::HistoryNotFound => (StatusCode::NOT_FOUND, String::from("History not found")),
            Self::EntryNotFound(entry_id) => (
                StatusCode::NOT_FOUND,
                format!("Entry not found: {entry_id}"),
            ),
            Self::RouteNotFound(pair_id) => (
                StatusCode::NOT_FOUND,
                format!("Missing data for routing on pair: {pair_id}"),
            ),
            Self::PublisherNotFound(publisher) => (
                StatusCode::NOT_FOUND,
                format!("Publisher not found: {publisher}"),
            ),
            Self::PublishData(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Can't publish data: {err}"),
            ),
            Self::PublisherError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Can't publish data: {err}"),
            ),
            Self::BuildPublish(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Can't build publish message: {err}"),
            ),
            Self::InvalidSigner => (StatusCode::BAD_REQUEST, "Could not sign price".to_string()),
            Self::InternalServerError(reason) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Interval server error: {reason}"),
            ),
            Self::WebSocketError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("WebSocket error: {err}"),
            ),
        };

        (
            status,
            Json(json!({
                "resource": "EntryModel",
                "message": err_msg,
                "happened_at": chrono::Utc::now()
            })),
        )
            .into_response()
    }
}
