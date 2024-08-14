use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use utoipa::ToSchema;

use crate::error::InfraError;

#[derive(Debug, thiserror::Error, ToSchema)]
pub enum OptimisticOracleError {
    #[error("internal server error")]
    InternalServerError,
    #[error("database connection error")]
    DatabaseConnection,
    #[error("assertion details issue: {0}")]
    AssertionDetailsIssue(String),
    #[error("disputer not set for assertion: {0}")]
    DisputerNotSet(String),
    #[error("settler not set for assertion: {0}")]
    SettlerNotSet(String),
    #[error("no assertions found for the given criteria")]
    NoAssertionsFound,
}

impl From<InfraError> for OptimisticOracleError {
    fn from(error: InfraError) -> Self {
        match error {
            InfraError::DisputerNotSet => Self::DisputerNotSet("Unknown".to_string()),
            InfraError::SettlerNotSet => Self::SettlerNotSet("Unknown".to_string()),
            _ => Self::InternalServerError,
        }
    }
}

impl IntoResponse for OptimisticOracleError {
    fn into_response(self) -> axum::response::Response {
        let (status, err_msg) = match self {
            Self::DatabaseConnection => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Could not establish a connection with the database".to_string(),
            ),
            Self::AssertionDetailsIssue(id) => (
                StatusCode::NOT_FOUND,
                format!("Issue to fetch assertion details with id: {}", id),
            ),
            Self::DisputerNotSet(id) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Disputer not set for assertion: {}", id),
            ),
            Self::SettlerNotSet(id) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Settler not set for assertion: {}", id),
            ),
            Self::NoAssertionsFound => (
                StatusCode::NOT_FOUND,
                "No assertions found for the given criteria".to_string(),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Internal server error"),
            ),
        };
        (
            status,
            Json(
                json!({"resource":"OptimisticOracle", "message": err_msg, "happened_at" : chrono::Utc::now() }),
            ),
        )
            .into_response()
    }
}
