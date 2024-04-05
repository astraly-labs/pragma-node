use axum::extract::{Query, State};
use axum::Json;

use crate::handlers::entries::{GetOHLCResponse, Interval};
use crate::infra::repositories::entry_repository::{self, OHLCEntry};
use crate::utils::PathExtractor;
use crate::AppState;
use pragma_entities::{error::InfraError, EntryError};

use super::utils::currency_pair_to_pair_id;
use super::GetEntryParams;

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
pub async fn get_ohlc(
    State(state): State<AppState>,
    PathExtractor(pair): PathExtractor<(String, String)>,
    Query(params): Query<GetEntryParams>,
) -> Result<Json<GetOHLCResponse>, EntryError> {
    tracing::info!("Received get entry request for pair {:?}", pair);
    // Construct pair id
    let pair_id = currency_pair_to_pair_id(&pair.0, &pair.1);

    let now = chrono::Utc::now().naive_utc().and_utc().timestamp_millis() as u64;

    let timestamp = if let Some(timestamp) = params.timestamp {
        timestamp
    } else {
        now
    };

    let interval = if let Some(interval) = params.interval {
        interval
    } else {
        Interval::OneMinute
    };

    // Validate given timestamp
    if timestamp > now {
        return Err(EntryError::InvalidTimestamp);
    }

    let entries = entry_repository::get_ohlc(&state.pool, pair_id.clone(), interval, timestamp)
        .await
        .map_err(|db_error| to_entry_error(db_error, &pair_id))?;

    Ok(Json(adapt_entry_to_entry_response(pair_id, &entries)))
}

fn adapt_entry_to_entry_response(pair_id: String, entries: &[OHLCEntry]) -> GetOHLCResponse {
    GetOHLCResponse {
        pair_id,
        data: entries.to_vec(),
    }
}

fn to_entry_error(error: InfraError, pair_id: &String) -> EntryError {
    match error {
        InfraError::InternalServerError => EntryError::InternalServerError,
        InfraError::NotFound => EntryError::NotFound(pair_id.to_string()),
        InfraError::InvalidTimeStamp => EntryError::InvalidTimestamp,
    }
}
