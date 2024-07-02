use axum::extract::State;
use axum::Json;
use chrono::NaiveDateTime;

use pragma_entities::EntryError;

use crate::handlers::entries::GetEntryResponse;
use crate::infra::repositories::entry_repository::{self, MedianEntry};
use crate::utils::PathExtractor;
use crate::AppState;

use super::GetEntryParams;
use crate::utils::{big_decimal_price_to_hex, currency_pair_to_pair_id};

#[utoipa::path(
    get,
    path = "/node/v1/data/{base}/{quote}/get_expiries",
    responses(
        (status = 200, description = "Get median entry successfuly", body = [GetEntryResponse])
    ),
    params(
        ("base" = String, Path, description = "Base Asset"),
        ("quote" = String, Path, description = "Quote Asset"),
        GetEntryParams,
    ),
)]
pub async fn get_expiries(
    State(state): State<AppState>,
    PathExtractor(pair): PathExtractor<(String, String)>,
) -> Result<Json<Vec<NaiveDateTime>>, EntryError> {
    tracing::info!("Received get entry request for pair {:?}", pair);
    // Construct pair id

    let pair_id = currency_pair_to_pair_id(&pair.0, &pair.1);

    let req_result = entry_repository::get_expiries_list(&state.offchain_pool, pair_id.clone())
        .await
        .map_err(|e| e.to_entry_error(&(pair_id)))?;

    tracing::info!("expiries are {:#?}", req_result);

    Ok(Json(req_result))
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
