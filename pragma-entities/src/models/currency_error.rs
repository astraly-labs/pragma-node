use crate::error::InfraError;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use utoipa::ToSchema;

#[derive(Debug, thiserror::Error, ToSchema)]
#[allow(unused)]
pub enum CurrencyError {
    #[error("internal server error")]
    InternalServerError,
    #[error("entry not found: {0}")]
    NotFound(String),
    #[error("infra error: {0}")]
    InfraError(InfraError),
}

impl IntoResponse for CurrencyError {
    fn into_response(self) -> axum::response::Response {
        let (status, err_msg) = match self {
            Self::NotFound(pair_id) => (
                StatusCode::NOT_FOUND,
                format!("CurrencyModel with pair id {} has not been found", pair_id),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Internal server error"),
            ),
        };
        (
            status,
            Json(
                json!({"resource":"CurrencyModel", "message": err_msg, "happened_at" : chrono::Utc::now() }),
            ),
        )
            .into_response()
    }
}
