use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

use pragma_entities::EntryError;

#[derive(Debug)]
#[allow(unused)]
pub enum AppError {
    InternalServerError,
    BodyParsingError(String),
    Entry(EntryError),
}

pub fn internal_error<E>(_err: E) -> AppError {
    AppError::InternalServerError
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, err_msg) = match self {
            Self::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Internal Server Error"),
            ),
            Self::BodyParsingError(message) => (
                StatusCode::BAD_REQUEST,
                format!("Bad request error: {message}"),
            ),
            Self::Entry(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Entry error: {err}"),
            ),
        };
        (status, Json(json!({ "message": err_msg }))).into_response()
    }
}
