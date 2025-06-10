use axum::Json;
use axum::extract::{Path, Query, State};
use pragma_common::Pair;
use pragma_entities::EntryError;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::infra::repositories::open_interest_repository;
use crate::state::AppState;

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetLatestOpenInterestParams {
    pub source: String,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GetLatestOpenInterestResponse {
    pub pair: String,
    pub source: String,
    pub timestamp_ms: u64,

    /// Open interest value quoted in the base asset
    pub open_interest: f64,
}

#[utoipa::path(
    get,
    path = "/node/v1/open_interest/{base}/{quote}",
    tag = "Open Interest",
    responses(
        (status = 200, description = "Successfully retrieved open interest", body = [GetLatestOpenInterestResponse]),
    ),
    params(
        ("base" = String, Path, description = "Base asset symbol (e.g., BTC)"),
        ("quote" = String, Path, description = "Quote asset symbol (e.g., USD)"),
        GetLatestOpenInterestParams
    )
)]
pub async fn get_latest_open_interest(
    State(state): State<AppState>,
    Path(pair): Path<(String, String)>,
    Query(params): Query<GetLatestOpenInterestParams>,
) -> Result<Json<GetLatestOpenInterestResponse>, EntryError> {
    let pair = Pair::from(pair);
    let source = params.source.to_ascii_uppercase();

    let open_interest = open_interest_repository::get_at_timestamp(
        &state.offchain_pool,
        pair.clone(),
        source,
        params.timestamp,
    )
    .await
    .map_err(EntryError::from)?
    .ok_or_else(|| EntryError::PairNotFound(pair.to_string()))?;

    let response = GetLatestOpenInterestResponse {
        pair: open_interest.pair,
        source: open_interest.source,
        timestamp_ms: open_interest.timestamp.and_utc().timestamp_millis() as u64,
        open_interest: open_interest.open_interest_value,
    };

    Ok(Json(response))
}
