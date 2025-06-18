use axum::Json;
use axum::extract::{Path, Query, State};
use pragma_common::Pair;
use pragma_entities::models::entries::timestamp::TimestampRange;
use pragma_entities::{EntryError, TimestampError};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use utoipa::{IntoParams, ToSchema};

use crate::infra::repositories::funding_rates_repository;
use crate::state::AppState;

use super::get_funding_rates::HOURS_IN_ONE_YEAR;

#[derive(Debug, Clone, Serialize, Deserialize, Display, EnumString, ToSchema)]
#[strum(serialize_all = "lowercase")]
pub enum Frequency {
    /// Return all data points
    All,
    /// Return data aggregated by minute (every minute)
    Minute,
    /// Return data aggregated by hour (every hour)
    Hour,
}

impl Default for Frequency {
    fn default() -> Self {
        Self::All
    }
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetHistoricalFundingRateParams {
    pub source: String,
    pub timestamp: TimestampRange,
    /// Frequency of data points (all, minute, hour). Defaults to 'all'
    #[serde(default)]
    pub frequency: Frequency,
    /// Page number (1-based). Defaults to 1
    #[serde(default = "default_page")]
    pub page: u64,
    /// Number of items per page. Defaults to 1000, max 10000
    #[serde(default = "default_page_size")]
    pub page_size: u64,
}

fn default_page() -> u64 {
    1
}

fn default_page_size() -> u64 {
    1000
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FundingRateResponse {
    pub pair: String,
    pub source: String,
    pub timestamp_ms: u64,
    pub hourly_rate: f64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GetHistoricalFundingRateResponse {
    pub data: Vec<FundingRateResponse>,
    pub page: u64,
    pub page_size: u64,
    pub has_next_page: bool,
}

#[utoipa::path(
    get,
    path = "/node/v1/funding_rates/history/{base}/{quote}",
    tag = "Historical Funding Rates",
    responses(
        (status = 200, description = "Successfully retrieved historical funding rates with pagination", body = GetHistoricalFundingRateResponse),
    ),
    params(
        ("base" = String, Path, description = "Base asset symbol (e.g., BTC)"),
        ("quote" = String, Path, description = "Quote asset symbol (e.g., USD)"),
        GetHistoricalFundingRateParams
    )
)]
pub async fn get_historical_funding_rates(
    State(state): State<AppState>,
    Path(pair): Path<(String, String)>,
    Query(params): Query<GetHistoricalFundingRateParams>,
) -> Result<Json<GetHistoricalFundingRateResponse>, EntryError> {
    let pair = Pair::from(pair);
    let source = params.source.to_ascii_uppercase();

    // Validate pagination parameters
    let page = if params.page == 0 { 1 } else { params.page };
    let page_size = params.page_size.min(10000).max(1); // Clamp between 1 and 10000

    let timestamp_range = params
        .timestamp
        .assert_time_is_valid()
        .map_err(|e| EntryError::InvalidTimestamp(TimestampError::RangeError(e)))?;

    let offset = (page - 1) * page_size;

    // Fetch one extra record to check if there's a next page
    let limit = page_size + 1;

    let funding_rates = funding_rates_repository::get_history_in_range_paginated(
        &state.offchain_pool,
        pair.clone(),
        source,
        timestamp_range,
        params.frequency,
        limit,
        offset,
    )
    .await
    .map_err(EntryError::from)?;

    // Check if we have more records than requested (indicates next page exists)
    let has_next_page = funding_rates.len() > page_size as usize;

    // Take only the requested number of records
    let data: Vec<FundingRateResponse> = funding_rates
        .into_iter()
        .take(page_size as usize)
        .map(|fr| FundingRateResponse {
            pair: fr.pair,
            source: fr.source,
            timestamp_ms: fr.timestamp.and_utc().timestamp_millis() as u64,
            hourly_rate: fr.annualized_rate / HOURS_IN_ONE_YEAR,
        })
        .collect();

    let response = GetHistoricalFundingRateResponse {
        data,
        page,
        page_size,
        has_next_page,
    };

    Ok(Json(response))
}
