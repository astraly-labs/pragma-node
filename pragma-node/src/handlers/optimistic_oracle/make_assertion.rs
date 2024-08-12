use axum::extract::State;
use axum::Json;
use chrono::Utc;

use crate::handlers::optimistic_oracle::types::{MakeAssertionRequest, Assertion};
use crate::infra::repositories::oo_repository::assertions;
use crate::AppState;

#[utoipa::path(
    post,
    path = "/assertions",
    request_body = MakeAssertionRequest,
    responses(
        (status = 201, description = "Assertion created successfully", body = MakeAssertionResponse)
    ),
)]
pub async fn make_assertion(
    State(state): State<AppState>,
    Json(request): Json<MakeAssertionRequest>,
) -> Result<Json<Assertion>, axum::http::StatusCode> {
    let assertion = assertions::create_assertion(
        &state.onchain_pool,
        request.claim,
        request.bond,
        request.expiration_time,
        request.identifier,
    )
    .await
    .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(assertion))
}
