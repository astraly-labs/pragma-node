use axum::Json;
use axum::extract::{Query, State};
use pragma_common::timestamp::{TimestampError, TimestampRangeError};
use pragma_common::types::pair::Pair;
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

use crate::handlers::Interval;
use crate::infra::repositories::entry_repository::{self, OHLCEntry};
use crate::state::AppState;
use crate::utils::PathExtractor;
use pragma_entities::EntryError;

use super::GetEntryParams;

/// Response containing OHLC (candlestick) data for a trading pair
#[derive(Debug, Serialize, Deserialize, ToSchema, ToResponse)]
#[schema(example = json!({
    "pair_id": "BTC/USD",
    "data": [
        {
            "time": "2025-03-10T07:30:00",
            "open": "82069269773700000000000",
            "low": "82023393045000000000000",
            "high": "82289627995410000000000",
            "close": "82208749021850000000000"
        }
    ]
}))]
pub struct GetOHLCResponse {
    /// Trading pair identifier (e.g., "BTC/USD")
    pub pair_id: String,

    /// Array of OHLC entries ordered by timestamp
    pub data: Vec<OHLCEntry>,
}

#[utoipa::path(
    get,
    path = "/node/v1/aggregation/candlestick/{base}/{quote}",
    tag = "Market Data",
    responses(
        (status = 200,
         description = "Successfully retrieved OHLC data", 
         body = GetOHLCResponse,
         example = json!({
             "pair_id": "BTC/USD",
             "data": [
                 {
                     "time": "2025-03-10T07:30:00",
                     "open": "82069269773700000000000",
                     "low": "82023393045000000000000",
                     "high": "82289627995410000000000",
                     "close": "82208749021850000000000"
                 }
             ]
         })
        ),
        (status = 400,
         description = "Invalid parameters", 
         body = EntryError,
         example = json!({
             "happened_at": "2025-03-10T08:27:29.324879945Z",
             "message": "Invalid timestamp: Timestamp range error: End timestamp is in the future",
             "resource": "EntryModel"
         })
        ),
        (status = 404,
         description = "No data found", 
         body = EntryError,
         example = json!({
             "happened_at": "2025-03-10T08:27:29.324879945Z",
             "message": "Entry not found",
             "resource": "EntryModel"
         })
        ),
        (status = 500,
         description = "Internal server error", 
         body = EntryError,
         example = json!({
             "happened_at": "2025-03-10T08:27:29.324879945Z",
             "message": "Database error: connection failed",
             "resource": "EntryModel"
         })
        )
    ),
    params(
        ("base" = String, Path, description = "Base asset symbol"),
        ("quote" = String, Path, description = "Quote asset symbol"),
        GetEntryParams,
    ),
    security(
        ("api_key" = [])
    )
)]
#[tracing::instrument(skip(state))]
pub async fn get_ohlc(
    State(state): State<AppState>,
    PathExtractor(pair): PathExtractor<(String, String)>,
    Query(params): Query<GetEntryParams>,
) -> Result<Json<GetOHLCResponse>, EntryError> {
    let pair = Pair::from(pair);

    let now = chrono::Utc::now().timestamp();

    let timestamp = params.timestamp.map_or(now, |timestamp| timestamp);

    let interval = params
        .interval
        .map_or(Interval::OneMinute, |interval| interval);

    // Validate given timestamp
    if timestamp > now {
        return Err(EntryError::InvalidTimestamp(TimestampError::RangeError(
            TimestampRangeError::EndInFuture,
        )));
    }

    let entries = entry_repository::get_spot_ohlc(
        &state.offchain_pool,
        pair.to_pair_id(),
        interval,
        timestamp,
    )
    .await
    .map_err(EntryError::from)?;

    Ok(Json(adapt_entry_to_entry_response(pair.into(), &entries)))
}

fn adapt_entry_to_entry_response(pair_id: String, entries: &[OHLCEntry]) -> GetOHLCResponse {
    GetOHLCResponse {
        pair_id,
        data: entries.to_vec(),
    }
}
