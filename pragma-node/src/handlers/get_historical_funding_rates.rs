use axum::Json;
use axum::extract::{Path, Query, State};
use pragma_entities::models::entries::timestamp::TimestampRange;
use pragma_entities::{EntryError, TimestampError};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::infra::repositories::funding_rates_repository;
use crate::state::AppState;

use super::get_funding_rates::HOURS_IN_ONE_YEAR;

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetHistoricalFundingRateParams {
    pub source: String,
    pub timestamp: TimestampRange,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FundingRateResponse {
    pub pair: String,
    pub source: String,
    pub timestamp: u64,
    pub hourly_rate: f64,
}

pub type GetHistoricalFundingRateResponse = Vec<FundingRateResponse>;

#[utoipa::path(
    get,
    path = "/node/v1/funding_rates/history/{base}/{quote}",
    tag = "Funding Rates",
    responses(
        (status = 200, description = "Successfully retrieved historical funding rates", body = [FundingRateResponse]),
    ),
    params(
        ("base" = String, Path, description = "Base asset symbol (e.g., BTC)"),
        ("quote" = String, Path, description = "Quote asset symbol (e.g., USD)"),
        GetHistoricalFundingRateParams
    )
)]
pub async fn get_historical_funding_rates(
    State(state): State<AppState>,
    Path((base, quote)): Path<(String, String)>,
    Query(params): Query<GetHistoricalFundingRateParams>,
) -> Result<Json<GetHistoricalFundingRateResponse>, EntryError> {
    let pair = format!("{base}/{quote}");
    let source = params.source.to_ascii_uppercase();

    let timestamp_range = params
        .timestamp
        .assert_time_is_valid()
        .map_err(|e| EntryError::InvalidTimestamp(TimestampError::RangeError(e)))?;

    let funding_rates = funding_rates_repository::get_history_in_range(
        &state.offchain_pool,
        pair.clone(),
        source,
        timestamp_range,
    )
    .await
    .map_err(EntryError::from)?;

    let response = funding_rates
        .into_iter()
        .map(|fr| FundingRateResponse {
            pair: fr.pair,
            source: fr.source,
            timestamp: fr.timestamp.and_utc().timestamp() as u64,
            hourly_rate: fr.annualized_rate / HOURS_IN_ONE_YEAR,
        })
        .collect();

    Ok(Json(response))
}
