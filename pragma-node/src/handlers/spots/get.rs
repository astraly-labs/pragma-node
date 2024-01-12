use axum::extract::{Query, State};
use axum::Json;
use bigdecimal::num_bigint::ToBigInt;

use crate::requests::{GetQueryParams, GetSpotResponse, SpotTimeStamp};
use crate::infra::repositories::entry_repository::{self, MedianEntry};
use crate::utils::PathExtractor;
use crate::AppState;
use pragma_entities::{EntryError, InfraError};

use crate::utils::{compute_median_price_and_time, currency_pair_to_pair_id};

#[utoipa::path(
get,
path = "/node/v1/data/spot/{asset1}/{asset2}",
responses(
(status = 200, description = "Get median entry successfuly", body = [GetSpotResponse])
),
params(
("quote" = String, Path, description = "Quote Asset"),
("base" = String, Path, description = "Base Asset")
),
)]
pub async fn get_spot(
    State(state): State<AppState>,
    PathExtractor(pair): PathExtractor<(String, String)>,
    params: Query<GetQueryParams>
) -> Result<Json<GetSpotResponse>, EntryError> {
    tracing::info!("Received get entry request for pair {:?}", pair);
    // Construct pair id
    let pair_id = currency_pair_to_pair_id(&pair.1, &pair.0);

    let now = chrono::Utc::now().naive_utc().timestamp_millis() as u64;

    let timestamp = if let Some(timestamp) = params.timestamp {
        timestamp
    } else {
        now
    };

    // Mock strk/eth pair
    if pair_id == "STRK/ETH" {
        return Ok(Json(GetSpotResponse {
            pair_id: "ETH/STRK".to_string(),
            data_range: SpotTimeStamp {
                start_timestamp: chrono::Utc::now().timestamp_millis() as u64,
                end_timestamp: chrono::Utc::now().timestamp_millis() as u64,
            },
            data: vec![],
            volume: 0,
            num_sources_aggregated: 5,
            price: "0x8ac7230489e80000".to_string(), // 0.1 wei
            decimals: 18,
        }));
    }

    // Get entries from database with given pair id (only the latest one grouped by publisher)
    let mut entries = entry_repository::get_entries_between(&state.pool, pair_id.clone(), now, timestamp)
        .await
        .map_err(|db_error| match db_error {
            _ => EntryError::InternalServerError,
            InfraError::NotFound => EntryError::NotFound(pair_id.clone()),
        })?;

    // Error if no entries found
    if entries.is_empty() {
        return Err(EntryError::UnknownPairId(pair_id));
    }

    let decimals = entry_repository::get_decimals(&state.pool, &pair_id)
        .await
        .map_err(|db_error| match db_error {
            _ => EntryError::InternalServerError,
            InfraError::NotFound => EntryError::NotFound(pair_id.clone()),
        })?;

    Ok(Json(adapt_entry_to_entry_response(
        pair_id,
        &mut entries,
        decimals,
    )))
}

fn adapt_entry_to_entry_response(
    pair_id: String,
    entries: &mut Vec<MedianEntry>,
    decimals: u32,
) -> GetSpotResponse {
    let (price, timestamp) = compute_median_price_and_time(entries).unwrap_or_default();

    GetSpotResponse {
        pair_id,
        data_range: SpotTimeStamp {
            start_timestamp: timestamp.timestamp_millis() as u64,
            end_timestamp: timestamp.timestamp_millis() as u64
        },
        num_sources_aggregated: entries.len(),
        price: format!("0x{}", price.to_bigint().unwrap().to_str_radix(16)),
        volume: 0,
        decimals,
        data: vec![],
    }
}
