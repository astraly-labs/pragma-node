use axum::extract::{Query, State};
use axum::Json;

use pragma_common::types::{AggregationMode, Interval};
use pragma_entities::EntryError;

use crate::handlers::entries::GetEntryResponse;
use crate::infra::repositories::entry_repository::{self, MedianEntry};
use crate::utils::PathExtractor;
use crate::AppState;

use super::utils::{big_decimal_price_to_hex, currency_pair_to_pair_id};
use super::GetEntryParams;

#[utoipa::path(
    get,
    path = "/node/v1/data/{base}/{quote}",
    responses(
        (status = 200, description = "Get median entry successfuly", body = [GetEntryResponse])
    ),
    params(
        ("base" = String, Path, description = "Base Asset"),
        ("quote" = String, Path, description = "Quote Asset"),
        GetEntryParams,
    ),
)]
pub async fn get_entry(
    State(state): State<AppState>,
    PathExtractor(pair): PathExtractor<(String, String)>,
    Query(params): Query<GetEntryParams>,
) -> Result<Json<GetEntryResponse>, EntryError> {
    tracing::info!("Received get entry request for pair {:?}", pair);
    // Construct pair id
    let pair_id = currency_pair_to_pair_id(&pair.0, &pair.1);

    let now = chrono::Utc::now().timestamp();

    let timestamp = if let Some(timestamp) = params.timestamp {
        timestamp
    } else {
        now
    };

    let interval = if let Some(interval) = params.interval {
        interval
    } else {
        Interval::TwoHours
    };

    let aggregation_mode = if let Some(aggregation_mode) = params.aggregation {
        aggregation_mode
    } else {
        AggregationMode::Twap
    };

    let is_routing = params.routing.unwrap_or(false);

    if timestamp > now {
        return Err(EntryError::InvalidTimestamp);
    }

    let (entry, decimals) = entry_repository::routing(
        &state.timescale_pool,
        pair_id.clone(),
        interval,
        timestamp,
        is_routing,
        aggregation_mode,
    )
    .await
    .map_err(|e| e.to_entry_error(&pair_id))?;

    Ok(Json(adapt_entry_to_entry_response(
        pair_id, &entry, decimals,
    )))
}

fn adapt_entry_to_entry_response(
    pair_id: String,
    entry: &MedianEntry,
    decimals: u32,
) -> GetEntryResponse {
    GetEntryResponse {
        pair_id,
        timestamp: entry.time.and_utc().timestamp_millis() as u64,
        num_sources_aggregated: entry.num_sources as usize,
        price: big_decimal_price_to_hex(&entry.median_price),
        decimals,
    }
}
