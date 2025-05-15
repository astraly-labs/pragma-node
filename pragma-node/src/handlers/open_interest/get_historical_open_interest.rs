use axum::Json;
use axum::extract::{Path, Query, State};
use pragma_common::Pair;
use pragma_entities::models::entries::timestamp::TimestampRange;
use pragma_entities::{EntryError, TimestampError};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::infra::repositories::open_interest_repository;
use crate::state::AppState;

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetHistoricalOpenInterestParams {
    pub source: String,
    pub timestamp: TimestampRange,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OpenInterestResponse {
    pub pair: String,
    pub source: String,
    pub timestamp_ms: u64,
    pub open_interest: f64,
}

pub type GetHistoricalOpenInterestResponse = Vec<OpenInterestResponse>;

#[utoipa::path(
    get,
    path = "/node/v1/open_interest/history/{base}/{quote}",
    tag = "Historical Open Interest",
    responses(
        (status = 200, description = "Successfully retrieved historical open interest", body = [OpenInterestResponse]),
    ),
    params(
        ("base" = String, Path, description = "Base asset symbol (e.g., BTC)"),
        ("quote" = String, Path, description = "Quote asset symbol (e.g., USD)"),
        GetHistoricalOpenInterestParams
    )
)]
pub async fn get_historical_open_interest(
    State(state): State<AppState>,
    Path(pair): Path<(String, String)>,
    Query(params): Query<GetHistoricalOpenInterestParams>,
) -> Result<Json<GetHistoricalOpenInterestResponse>, EntryError> {
    let pair = Pair::from(pair);
    let source = params.source.to_ascii_uppercase();

    let timestamp_range = params
        .timestamp
        .assert_time_is_valid()
        .map_err(|e| EntryError::InvalidTimestamp(TimestampError::RangeError(e)))?;

    let open_interests = open_interest_repository::get_history_in_range(
        &state.offchain_pool,
        pair.clone(),
        source,
        timestamp_range,
    )
    .await
    .map_err(EntryError::from)?;

    let response = open_interests
        .into_iter()
        .map(|oi| OpenInterestResponse {
            pair: oi.pair,
            source: oi.source,
            timestamp_ms: oi.timestamp.and_utc().timestamp_millis() as u64,
            open_interest: oi.open_interest_value,
        })
        .collect();

    Ok(Json(response))
} 