use std::{collections::HashMap, sync::Arc, time::Duration};

use deadpool_diesel::postgres::Pool;
use tokio::sync::mpsc;

use pragma_entities::{NewEntry, NewFundingRate, NewFutureEntry, NewOpenInterest};

use crate::db::insert::{
    insert_funding_rate_entries, insert_future_entries, insert_open_interest_entries,
    insert_spot_entries,
};
use crate::metrics::{IngestorMetricsRegistry, InsertDbOperation, Status};

const PUBLISHING_DELAY: Duration = Duration::from_millis(500);
const MINUTE_PUBLISHING_DELAY: Duration = Duration::from_secs(60);

/// Sources allowed for EUR/USD pair ingestion.
/// Only PYTH and LMAX are authorized for this forex pair.
const EURUSD_ALLOWED_SOURCES: &[&str] = &["PYTH", "LMAX"];

/// Checks if an entry should be ingested based on pair/source filtering rules.
/// Returns false (skip) for EUR/USD entries from non-allowed sources.
fn should_ingest_entry(pair_id: &str, source: &str) -> bool {
    if pair_id == "EUR/USD" {
        let source_upper = source.to_uppercase();
        EURUSD_ALLOWED_SOURCES
            .iter()
            .any(|allowed| *allowed == source_upper)
    } else {
        true
    }
}

#[tracing::instrument(skip(pool, rx, metrics_registry))]
pub(crate) async fn process_spot_entries(
    pool: Pool,
    mut rx: mpsc::Receiver<NewEntry>,
    metrics_registry: Arc<IngestorMetricsRegistry>,
) {
    let mut batched_data = HashMap::new();
    let mut interval = tokio::time::interval(PUBLISHING_DELAY);

    loop {
        tokio::select! {
            Some(entry) = rx.recv() => {
                // Filter EUR/USD to only allow PYTH and LMAX sources
                if !should_ingest_entry(&entry.pair_id, &entry.source) {
                    continue;
                }
                let key = (entry.source.clone(), entry.pair_id.clone());
                batched_data.insert(key, entry);
            },
            _ = interval.tick() => {
                let batch = std::mem::take(&mut batched_data);
                let batch = batch.into_values().collect();
                match insert_spot_entries(&pool, batch).await {
                    Ok(()) => {
                        metrics_registry.record_db_operation(InsertDbOperation::SpotEntries, Status::Success);
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to insert spot entries: {e:?}");
                        metrics_registry.record_db_operation(InsertDbOperation::SpotEntries, Status::Error);
                    }
                }
            }
        }
    }
}

#[tracing::instrument(skip(pool, rx, metrics_registry))]
pub(crate) async fn process_future_entries(
    pool: Pool,
    mut rx: mpsc::Receiver<NewFutureEntry>,
    metrics_registry: Arc<IngestorMetricsRegistry>,
) {
    let mut batched_data = HashMap::new();
    let mut interval = tokio::time::interval(PUBLISHING_DELAY);

    loop {
        tokio::select! {
            Some(entry) = rx.recv() => {
                let key = (entry.source.clone(), entry.pair_id.clone());
                batched_data.insert(key, entry);
            },
            _ = interval.tick() => {
                let batch = std::mem::take(&mut batched_data);
                let batch = batch.into_values().collect();
                match insert_future_entries(&pool, batch).await {
                    Ok(()) => {
                        metrics_registry.record_db_operation(InsertDbOperation::FutureEntries, Status::Success);
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to insert future entries: {e:?}");
                        metrics_registry.record_db_operation(InsertDbOperation::FutureEntries, Status::Error);
                    }
                }
            }
        }
    }
}

#[tracing::instrument(skip(pool, rx, metrics_registry))]
pub(crate) async fn process_funding_rate_entries(
    pool: Pool,
    mut rx: mpsc::Receiver<NewFundingRate>,
    metrics_registry: Arc<IngestorMetricsRegistry>,
) {
    let mut batched_data = HashMap::new();
    let mut interval = tokio::time::interval(MINUTE_PUBLISHING_DELAY);

    loop {
        tokio::select! {
            Some(entry) = rx.recv() => {
                let key = (
                    entry.source.clone(),
                    entry.pair.clone(),
                    entry.timestamp.and_utc().timestamp_millis() / (60 * 1000) // Convert ms to minutes
                );
                // Only insert if we don't have an entry for this minute yet
                batched_data.entry(key).or_insert(entry);
            },
            _ = interval.tick() => {
                let batch = std::mem::take(&mut batched_data);
                let batch = batch.into_values().collect();
                match insert_funding_rate_entries(&pool, batch).await {
                    Ok(()) => {
                        metrics_registry.record_db_operation(InsertDbOperation::FundingRates, Status::Success);
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to insert funding rates: {e:?}");
                        metrics_registry.record_db_operation(InsertDbOperation::FundingRates, Status::Error);
                    }
                }
            }
        }
    }
}

#[tracing::instrument(skip(pool, rx, metrics_registry))]
pub(crate) async fn process_open_interest_entries(
    pool: Pool,
    mut rx: mpsc::Receiver<NewOpenInterest>,
    metrics_registry: Arc<IngestorMetricsRegistry>,
) {
    let mut batched_data = HashMap::new();
    let mut interval = tokio::time::interval(MINUTE_PUBLISHING_DELAY);

    loop {
        tokio::select! {
            Some(entry) = rx.recv() => {
                let key = (
                    entry.source.clone(),
                    entry.pair.clone(),
                    entry.timestamp.and_utc().timestamp_millis() / (60 * 1000) // Convert ms to minutes
                );
                // Only insert if we don't have an entry for this minute yet
                batched_data.entry(key).or_insert(entry);
            },
            _ = interval.tick() => {
                let batch = std::mem::take(&mut batched_data);
                let batch = batch.into_values().collect();
                match insert_open_interest_entries(&pool, batch).await {
                    Ok(()) => {
                        metrics_registry.record_db_operation(InsertDbOperation::OpenInterest, Status::Success);
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to insert open interest entries: {e:?}");
                        metrics_registry.record_db_operation(InsertDbOperation::OpenInterest, Status::Error);
                    }
                }
            }
        }
    }
}
