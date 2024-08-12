use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::handlers::optimistic_oracle::types::{
    DisputeAssertionRequest, DisputeAssertionResponse,
};
use crate::infra::repositories::oo_repository::assertions;
use crate::AppState;

#[utoipa::path(
    post,
    path = "/assertions/{assertion_id}/disputes",
    request_body = DisputeAssertionRequest,
    responses(
        (status = 201, description = "Assertion disputed successfully", body = DisputeAssertionResponse)
    ),
    params(
        ("assertion_id" = String, Path, description = "Unique identifier of the assertion to dispute"),
    ),
)]
pub async fn dispute_assertion(
    State(state): State<AppState>,
    Path(assertion_id): Path<String>,
    Json(request): Json<DisputeAssertionRequest>,
) -> Result<Json<DisputeAssertionResponse>, axum::http::StatusCode> {
    let dispute =
        assertions::create_dispute(&state.onchain_pool, &assertion_id, request.dispute_bond)
            .await
            .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(dispute))
}
