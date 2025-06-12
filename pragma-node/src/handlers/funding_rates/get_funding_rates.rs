use axum::Json;
use axum::extract::{Path, Query, State};
use pragma_common::Pair;
use pragma_entities::EntryError;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::infra::repositories::funding_rates_repository;
use crate::state::AppState;

pub const HOURS_IN_ONE_YEAR: f64 = 8760.0;

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetLatestFundingRateParams {
    pub source: String,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GetLatestFundingRateResponse {
    pub pair: String,
    pub source: String,
    pub timestamp_ms: u64,
    pub hourly_rate: f64,
}

#[utoipa::path(
    get,
    path = "/node/v1/funding_rates/{base}/{quote}",
    tag = "Funding Rates",
    responses(
        (status = 200, description = "Successfully retrieved funding rate", body = [GetLatestFundingRateResponse]),
    ),
    params(
        ("base" = String, Path, description = "Base asset symbol (e.g., BTC)"),
        ("quote" = String, Path, description = "Quote asset symbol (e.g., USD)"),
        GetLatestFundingRateParams
    )
)]
pub async fn get_latest_funding_rate(
    State(state): State<AppState>,
    Path(pair): Path<(String, String)>,
    Query(params): Query<GetLatestFundingRateParams>,
) -> Result<Json<GetLatestFundingRateResponse>, EntryError> {
    let pair = Pair::from(pair);
    let source = params.source.to_ascii_uppercase();

    let funding_rate = funding_rates_repository::get_at_timestamp(
        &state.offchain_pool,
        pair.clone(),
        source,
        params.timestamp,
    )
    .await
    .map_err(EntryError::from)?
    .ok_or_else(|| EntryError::PairNotFound(pair.to_string()))?;

    let response = GetLatestFundingRateResponse {
        pair: funding_rate.pair,
        source: funding_rate.source,
        timestamp_ms: funding_rate.timestamp.and_utc().timestamp_millis() as u64,
        hourly_rate: funding_rate.annualized_rate / HOURS_IN_ONE_YEAR,
    };

    Ok(Json(response))
}
