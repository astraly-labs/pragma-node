use axum::extract::{Query, State};
use axum::Json;
use bigdecimal::num_bigint::ToBigInt;

use crate::requests::{GetPerpResponse, GetQueryParams};
use crate::infra::errors::InfraError;
use crate::infra::repositories::entry_repository::{self, MedianEntry};
use crate::utils::PathExtractor;
use crate::AppState;
use pragma_entities::EntryError;

use crate::utils::{compute_median_price_and_time, currency_pair_to_pair_id};

#[utoipa::path(
get,
path = "/node/v1/data/perp/{asset1}/{asset2}",
responses(
(status = 200, description = "Get median entry successfuly", body = [GetPerpResponse])
),
params(
("quote" = String, Path, description = "Quote Asset"),
("base" = String, Path, description = "Base Asset")
),
)]
pub async fn get_perp(
    State(state): State<AppState>,
    PathExtractor(pair): PathExtractor<(String, String)>,
    params: Query<GetQueryParams>
) -> Result<Json<GetPerpResponse>, EntryError> {
    tracing::info!("Received get entry request for pair {:?}", pair);
    // Construct pair id
    let pair_id = currency_pair_to_pair_id(&pair.1, &pair.0);

    // Mock strk/eth pair
    if pair_id == "STRK/ETH" {
        return Ok(Json(GetPerpResponse {
            pair_id: "ETH/STRK".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            num_sources_aggregated: 5,
            funding_rate: 0,
            basis: 0,
            open_interest: 0,
            volume: 0,
            price: "0x8ac7230489e80000".to_string(), // 0.1 wei
            decimals: 18,
        }));
    }

    // Get entries from database with given pair id (only the latest one grouped by publisher)
    let mut entries = entry_repository::get_median_entries(&state.pool, pair_id.clone())
        .await
        .map_err(|db_error| match db_error {
            InfraError::InternalServerError => EntryError::InternalServerError,
            InfraError::NotFound => EntryError::NotFound(pair_id.clone()),
        })?;

    // Error if no entries found
    if entries.is_empty() {
        return Err(EntryError::UnknownPairId(pair_id));
    }

    let decimals = entry_repository::get_decimals(&state.pool, &pair_id)
        .await
        .map_err(|db_error| match db_error {
            InfraError::InternalServerError => EntryError::InternalServerError,
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
) -> GetPerpResponse {
    let (price, timestamp) = compute_median_price_and_time(entries).unwrap_or_default();

    GetPerpResponse {
        pair_id,
        timestamp: timestamp.timestamp_millis() as u64,
        volume: 0,
        basis: 0,
        open_interest: 0,
        num_sources_aggregated: entries.len(),
        price: format!("0x{}", price.to_bigint().unwrap().to_str_radix(16)),
        decimals,
        funding_rate: 0,
    }
}
