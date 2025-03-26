use std::pin::Pin;

use axum::response::sse::Event;
use pragma_common::types::{AggregationMode, pair::Pair};
use pragma_entities::EntryError;

use crate::{
    handlers::get_entry::{EntryParams, GetEntryResponse, adapt_entry_to_entry_response},
    infra::repositories::entry_repository,
    state::AppState,
};

pub const DEFAULT_HISTORICAL_PRICES: usize = 50;

pub type BoxedFuture = Pin<Box<dyn Future<Output = Event> + Send>>;
pub type BoxedStreamItem = Box<dyn FnMut() -> BoxedFuture + Send>;

pub async fn get_historical_entries(
    state: &AppState,
    pair: &Pair,
    entry_params: &EntryParams,
    count: usize,
) -> Result<Vec<GetEntryResponse>, EntryError> {
    let interval = entry_params.interval;
    // Get current timestamp
    let end_timestamp = chrono::Utc::now().timestamp() as u64;
    // Get timestamp from count minutes ago
    let start_timestamp = end_timestamp.saturating_sub(count as u64 * interval.to_seconds() as u64);

    // Get entries based on aggregation mode
    let entries = match entry_params.aggregation_mode {
        AggregationMode::Median => entry_repository::get_median_prices_between(
            &state.offchain_pool,
            pair.to_pair_id(),
            entry_params.clone(),
            start_timestamp,
            end_timestamp,
        )
        .await
        .map_err(EntryError::from)?,
        AggregationMode::Twap => unreachable!(),
    };

    let responses: Vec<GetEntryResponse> = entries
        .into_iter()
        .take(count)
        .map(|entry| adapt_entry_to_entry_response(pair.to_pair_id(), &entry, entry.time))
        .collect();

    Ok(responses)
}

pub async fn get_latest_entry(
    state: &AppState,
    pair: &Pair,
    is_routing: bool,
    entry_params: &EntryParams,
    with_components: bool,
) -> Result<GetEntryResponse, EntryError> {
    // We have to update the timestamp to now every tick
    let mut new_routing = entry_params.clone();
    new_routing.timestamp = chrono::Utc::now().timestamp();

    let entry = entry_repository::routing(
        &state.offchain_pool,
        is_routing,
        pair,
        &new_routing,
        with_components,
    )
    .await
    .map_err(EntryError::from)?;

    let last_updated_timestamp = entry_repository::get_last_updated_timestamp(
        &state.offchain_pool,
        pair.to_pair_id(),
        new_routing.timestamp,
    )
    .await?
    .unwrap_or(entry.time);

    Ok(adapt_entry_to_entry_response(
        pair.to_pair_id(),
        &entry,
        last_updated_timestamp,
    ))
}

pub async fn get_historical_entries_multi_pair(
    state: &AppState,
    pairs: &[Pair],
    entry_params: &EntryParams,
    count: usize,
) -> Result<Vec<Vec<GetEntryResponse>>, EntryError> {
    let mut all_entries = Vec::with_capacity(pairs.len());

    for pair in pairs {
        match get_historical_entries(state, pair, entry_params, count).await {
            Ok(entries) => all_entries.push(entries),
            Err(e) => {
                tracing::warn!(
                    "Failed to get historical entries for pair {}: {}",
                    pair.to_pair_id(),
                    e
                );
                // Skip this pair and continue with others
                continue;
            }
        }
    }

    // Return error only if we couldn't get any entries
    if all_entries.is_empty() {
        return Err(EntryError::HistoryNotFound);
    }

    Ok(all_entries)
}

pub async fn get_latest_entries_multi_pair(
    state: &AppState,
    pairs: &[Pair],
    is_routing: bool,
    entry_params: &EntryParams,
    with_components: bool,
) -> Result<Vec<GetEntryResponse>, EntryError> {
    let mut latest_entries = Vec::with_capacity(pairs.len());

    for pair in pairs {
        match get_latest_entry(state, pair, is_routing, entry_params, with_components).await {
            Ok(entry) => latest_entries.push(entry),
            Err(e) => {
                tracing::warn!(
                    "Failed to get latest entry for pair {}: {}",
                    pair.to_pair_id(),
                    e
                );
                // Skip this pair and continue with others
                continue;
            }
        }
    }

    // Return error only if we couldn't get any entries
    if latest_entries.is_empty() {
        return Err(EntryError::HistoryNotFound);
    }

    Ok(latest_entries)
}
