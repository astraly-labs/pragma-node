use axum::Json;
use axum::extract::{Path, Query, State};
use pragma_common::Pair;
use pragma_entities::models::entries::timestamp::TimestampRange;
use pragma_entities::{EntryError, TimestampError};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use utoipa::{IntoParams, ToSchema};

use pragma_entities::{PaginationParams, PaginationResponse};

use crate::constants::others::HOURS_IN_ONE_YEAR;
use crate::infra::repositories::funding_rates_repository;
use crate::state::AppState;

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
    /// Pagination parameters
    #[serde(flatten)]
    pub pagination: PaginationParams,
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
    #[serde(flatten)]
    pub pagination: PaginationResponse,
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
        ("source" = String, Query, description = "Source of the funding rates (e.g., bybit, hyperliquid, paradex)"),
        ("timestamp" = TimestampRange, Query, description = "Timestamp range (e.g., 1718745600000,1718832000000)"),
        ("frequency" = Frequency, Query, description = "Frequency of the data points (all, minute, hour)"),
        ("page" = i64, Query, description = "Page number (1-based)"),
        ("page_size" = i64, Query, description = "Number of items per page (1-1000)"),
    )
)]
pub async fn get_historical_funding_rates(
    State(state): State<AppState>,
    Path(pair): Path<(String, String)>,
    Query(params): Query<GetHistoricalFundingRateParams>,
) -> Result<Json<GetHistoricalFundingRateResponse>, EntryError> {
    let pair = Pair::from(pair);
    let source = params.source.to_ascii_uppercase();

    // Validate pagination parameters using the new helper methods
    let page = params.pagination.page();
    let page_size = params.pagination.page_size();

    let timestamp_range = params
        .timestamp
        .assert_time_is_valid()
        .map_err(|e| EntryError::InvalidTimestamp(TimestampError::RangeError(e)))?;

    let funding_rates = funding_rates_repository::get_history_in_range_paginated(
        &state.offchain_pool,
        pair.clone(),
        source,
        timestamp_range,
        params.frequency,
        params.pagination,
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
        pagination: PaginationResponse::new(page, page_size, has_next_page),
    };

    Ok(Json(response))
}
