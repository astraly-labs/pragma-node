use axum::extract::{Query, State};
use axum::Json;
use bigdecimal::num_bigint::ToBigInt;

use crate::requests::GetEntryResponse;
use crate::handlers::entries::Interval;
use crate::infra::repositories::entry_repository::{self, MedianEntry};
use crate::utils::PathExtractor;
use crate::AppState;
use pragma_entities::{error::InfraError, EntryError};

use super::utils::currency_pair_to_pair_id;
use super::GetEntryParams;

#[utoipa::path(
        get,
        path = "/node/v1/data/{base}/{quote}",
        responses(
            (status = 200, description = "Get median entry successfuly", body = [GetEntryResponse])
        ),
        params(
            ("quote" = String, Path, description = "Quote Asset"),
            ("base" = String, Path, description = "Base Asset"),
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
    let pair_id = currency_pair_to_pair_id(&pair.1, &pair.0);

    let now = chrono::Utc::now().naive_utc().timestamp_millis() as u64;

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

    // Mock strk/eth pair
    if pair_id == "STRK/ETH" {
        return Ok(Json(GetEntryResponse {
            pair_id: "ETH/STRK".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            num_sources_aggregated: 5,
            price: "0x8ac7230489e80000".to_string(), // 0.1 wei
            decimals: 18,
        }));
    }

    let entry =
        entry_repository::get_median_price(&state.pool, pair_id.clone(), interval, timestamp)
            .await
            .map_err(|db_error| to_entry_error(db_error, &pair_id))?;

    let decimals = entry_repository::get_decimals(&state.pool, &pair_id)
        .await
        .map_err(|db_error| to_entry_error(db_error, &pair_id))?;

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
        timestamp: entry.time.timestamp_millis() as u64,
        num_sources_aggregated: entry.num_sources as usize,
        price: format!(
            "0x{}",
            entry
                .median_price
                .to_bigint()
                .unwrap_or_default()
                .to_str_radix(16)
        ),
        decimals,
    }
}

fn to_entry_error(error: InfraError, pair_id: &String) -> EntryError {
    match error {
        InfraError::InternalServerError => EntryError::InternalServerError,
        InfraError::NotFound => EntryError::NotFound(pair_id.to_string()),
        InfraError::InvalidTimeStamp => EntryError::InvalidTimestamp,
    }
}
