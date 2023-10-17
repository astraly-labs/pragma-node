use axum::extract::State;
use axum::Json;
use bigdecimal::{BigDecimal, ToPrimitive};
use chrono::NaiveDateTime;

use crate::domain::models::entry::EntryError;
use crate::handlers::entries::GetEntryResponse;
use crate::infra::errors::InfraError;
use crate::infra::repositories::entry_repository::{self, MedianEntry};
use crate::utils::PathExtractor;
use crate::AppState;

/// Converts a currency pair to a pair id.
fn currency_pair_to_pair_id(quote: &str, base: &str) -> String {
    format!("{}/{}", quote.to_uppercase(), base.to_uppercase())
}

pub async fn get_entry(
    State(state): State<AppState>,
    PathExtractor(pair): PathExtractor<(String, String)>,
) -> Result<Json<GetEntryResponse>, EntryError> {
    tracing::info!("Received get entry request for pair {:?}", pair);
    // Construct pair id
    let pair_id = currency_pair_to_pair_id(&pair.0, &pair.1);

    // Get entries from database with given pair id (only the latest one grouped by publisher)
    let mut entries = entry_repository::get_median_entries(&state.pool, pair_id.clone())
        .await
        .map_err(|db_error| match db_error {
            InfraError::InternalServerError => EntryError::InternalServerError,
            InfraError::NotFound => EntryError::NotFound(pair_id.clone()),
        })?;

    Ok(Json(adapt_entry_to_entry_response(pair_id, &mut entries)))
}

fn adapt_entry_to_entry_response(
    pair_id: String,
    entries: &mut Vec<MedianEntry>,
) -> GetEntryResponse {
    let (price, timestamp) = compute_median_price_and_time(entries).unwrap_or_default();

    GetEntryResponse {
        pair_id,
        timestamp: timestamp.timestamp() as u64,
        num_sources_aggregated: 0, // TODO: add real value
        price: price.to_u128().unwrap(),
    }
}

fn compute_median_price_and_time(
    entries: &mut Vec<MedianEntry>,
) -> Option<(BigDecimal, NaiveDateTime)> {
    if entries.is_empty() {
        return None;
    }

    // Compute median price
    entries.sort_by(|a, b| a.median_price.cmp(&b.median_price));
    let mid = entries.len() / 2;
    let median_price = if entries.len() % 2 == 0 {
        (&entries[mid - 1].median_price + &entries[mid].median_price) / BigDecimal::from(2)
    } else {
        entries[mid].median_price.clone()
    };

    // Compute median time
    entries.sort_by(|a, b| a.time.cmp(&b.time));
    let median_time = if entries.len() % 2 == 0 {
        entries[mid - 1].time
    } else {
        entries[mid].time
    };

    Some((median_price, median_time))
}
