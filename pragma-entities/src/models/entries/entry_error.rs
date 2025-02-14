use crate::error::InfraError;
use crate::models::publisher_error::PublisherError;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use pragma_common::signing::SignerError;
use pragma_common::timestamp::TimestampRangeError;
use pragma_common::types::{AggregationMode, Interval};
use serde_json::json;
use starknet::core::crypto::EcdsaVerifyError;
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
    #[schema(value_type = String)]
    InvalidSignature(EcdsaVerifyError),
    #[error("could not sign price")]
    InvalidSigner,
    #[error("unauthorized request: {0}")]
    Unauthorized(String),
    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(#[from] TimestampRangeError),
    #[error("invalid expiry")]
    InvalidExpiry,
    #[error("missing data for routing on pair: {0}")]
    MissingData(String),
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
    #[error(transparent)]
    SignerError(#[from] SignerError),
    #[error("invalid login message: {0}")]
    InvalidLoginMessage(String),
    #[error("unsupported interval {0:?} for aggregation {1:?}")]
    UnsupportedInterval(Interval, AggregationMode),
}

impl From<InfraError> for EntryError {
    fn from(error: InfraError) -> Self {
        match error {
            InfraError::NotFound => Self::NotFound("Unknown".to_string()),
            InfraError::RoutingError => Self::MissingData("Not enough data".to_string()),
            InfraError::InvalidTimestamp(e) => {
                Self::InvalidTimestamp(TimestampRangeError::Other(e))
            }
            InfraError::UnsupportedInterval(i, a) => Self::UnsupportedInterval(i, a),
            InfraError::InternalServerError
            | InfraError::DisputerNotSet
            | InfraError::SettlerNotSet
            | InfraError::NonZeroU32Conversion(_)
            | InfraError::AxumError(_) => Self::InternalServerError,
        }
    }
}

impl IntoResponse for EntryError {
    fn into_response(self) -> axum::response::Response {
        let (status, err_msg) = match self {
            Self::NotFound(pair_id) => (
                StatusCode::NOT_FOUND,
                format!("EntryModel with pair id {pair_id} has not been found"),
            ),
            Self::MissingData(pair_id) => (
                StatusCode::NOT_FOUND,
                format!("Not enough data on pair {pair_id} to perform routing"),
            ),
            Self::InfraError(db_error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal server error: {db_error}"),
            ),
            Self::InvalidSignature(err) => {
                (StatusCode::BAD_REQUEST, format!("Invalid signature: {err}"))
            }
            Self::Unauthorized(reason) => (
                StatusCode::UNAUTHORIZED,
                format!("Unauthorized publisher: {reason}"),
            ),
            Self::InvalidTimestamp(reason) => (
                StatusCode::BAD_REQUEST,
                format!("Invalid timestamp: {reason}"),
            ),
            Self::InvalidExpiry => (StatusCode::BAD_REQUEST, "Invalid expiry".to_string()),
            Self::PublisherError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Publisher error: {err}"),
            ),
            Self::PublishData(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Unable to publish data: {err}"),
            ),
            Self::BuildPublish(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Unable to build publish message: {err}"),
            ),
            Self::BadRequest => (StatusCode::BAD_REQUEST, "Bad request".to_string()),
            Self::UnknownPairId(pair_id) => {
                (StatusCode::NOT_FOUND, format!("Unknown pair id: {pair_id}"))
            }
            Self::SignerError(err) => (StatusCode::BAD_REQUEST, format!("Invalid message: {err}")),
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
