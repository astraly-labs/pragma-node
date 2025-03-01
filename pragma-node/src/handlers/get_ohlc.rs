use axum::Json;
use axum::extract::{Query, State};
use pragma_common::timestamp::TimestampRangeError;
use pragma_common::types::pair::Pair;
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

use crate::AppState;
use crate::handlers::Interval;
use crate::infra::repositories::entry_repository::{self, OHLCEntry};
use crate::utils::PathExtractor;
use pragma_entities::EntryError;

use super::GetEntryParams;

#[derive(Debug, Serialize, Deserialize, ToSchema, ToResponse)]
pub struct GetOHLCResponse {
    pair_id: String,
    data: Vec<OHLCEntry>,
}

#[utoipa::path(
        get,
        path = "/node/v1/aggregation/candlestick/{base}/{quote}",
        responses(
            (status = 200, description = "Get OHLC data successfuly", body = [GetOHLCResponse])
        ),
        params(
            ("base" = String, Path, description = "Base Asset"),
            ("quote" = String, Path, description = "Quote Asset"),
            GetEntryParams,
        ),
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
        return Err(EntryError::InvalidTimestamp(
            TimestampRangeError::EndInFuture,
        ));
    }

    let entries =
        entry_repository::get_ohlc(&state.offchain_pool, pair.to_pair_id(), interval, timestamp)
            .await
            .map_err(|db_error| db_error.to_entry_error(&pair.to_pair_id()))?;

    Ok(Json(adapt_entry_to_entry_response(pair.into(), &entries)))
}

fn adapt_entry_to_entry_response(pair_id: String, entries: &[OHLCEntry]) -> GetOHLCResponse {
    GetOHLCResponse {
        pair_id,
        data: entries.to_vec(),
    }
}
