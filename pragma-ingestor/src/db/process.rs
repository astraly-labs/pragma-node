use std::{collections::HashMap, sync::Arc, time::Duration};

use deadpool_diesel::postgres::Pool;
use tokio::sync::mpsc;

use pragma_entities::{NewEntry, NewFundingRate, NewFutureEntry, NewOpenInterest};

use crate::db::insert::{
    insert_funding_rate_entries, insert_future_entries, insert_open_interest_entries,
    insert_spot_entries,
};
use crate::metrics::{DbOperation, IngestorMetricsRegistry, Status};

const PUBLISHING_DELAY: Duration = Duration::from_millis(50);
const MINUTE_PUBLISHING_DELAY: Duration = Duration::from_secs(60);

#[tracing::instrument(skip(pool, rx, metrics_registry))]
pub async fn process_spot_entries(
    pool: Pool,
    mut rx: mpsc::Receiver<NewEntry>,
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
                match insert_spot_entries(&pool, batch).await {
                    Ok(_) => {
                        metrics_registry.record_db_operation(DbOperation::InsertSpotEntries, Status::Success);
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to insert spot entries: {e:?}");
                        metrics_registry.record_db_operation(DbOperation::InsertSpotEntries, Status::Error);
                    }
                }
            }
        }
    }
}

#[tracing::instrument(skip(pool, rx, metrics_registry))]
pub async fn process_future_entries(
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
                    Ok(_) => {
                        metrics_registry.record_db_operation(DbOperation::InsertFutureEntries, Status::Success);
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to insert future entries: {e:?}");
                        metrics_registry.record_db_operation(DbOperation::InsertFutureEntries, Status::Error);
                    }
                }
            }
        }
    }
}

#[tracing::instrument(skip(pool, rx, metrics_registry))]
pub async fn process_funding_rate_entries(
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
                    Ok(_) => {
                        metrics_registry.record_db_operation(DbOperation::InsertFundingRates, Status::Success);
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to insert funding rates: {e:?}");
                        metrics_registry.record_db_operation(DbOperation::InsertFundingRates, Status::Error);
                    }
                }
            }
        }
    }
}

#[tracing::instrument(skip(pool, rx, metrics_registry))]
pub async fn process_open_interest_entries(
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
                    Ok(_) => {
                        metrics_registry.record_db_operation(DbOperation::InsertOpenInterest, Status::Success);
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to insert open interest entries: {e:?}");
                        metrics_registry.record_db_operation(DbOperation::InsertOpenInterest, Status::Error);
                    }
                }
            }
        }
    }
}
