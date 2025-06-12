use std::{collections::HashMap, time::Duration};

use deadpool_diesel::postgres::Pool;
use tokio::sync::mpsc;

use pragma_entities::{NewEntry, NewFundingRate, NewFutureEntry, NewOpenInterest};

use crate::db::insert::{
    insert_funding_rate_entries, insert_future_entries, insert_open_interest_entries,
    insert_spot_entries,
};

const PUBLISHING_DELAY: Duration = Duration::from_millis(50);

#[tracing::instrument(skip(pool, rx))]
pub async fn process_spot_entries(pool: Pool, mut rx: mpsc::Receiver<NewEntry>) {
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
                if let Err(e) = insert_spot_entries(&pool, batch).await {
                    tracing::error!("❌ Failed to insert spot entries: {e:?}");
                }
            }
        }
    }
}

#[tracing::instrument(skip(pool, rx))]
pub async fn process_future_entries(pool: Pool, mut rx: mpsc::Receiver<NewFutureEntry>) {
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
                if let Err(e) = insert_future_entries(&pool, batch).await {
                    tracing::error!("❌ Failed to insert future entries: {e:?}");
                }
            }
        }
    }
}

#[tracing::instrument(skip(pool, rx))]
pub async fn process_funding_rate_entries(pool: Pool, mut rx: mpsc::Receiver<NewFundingRate>) {
    let mut batched_data = HashMap::new();
    let mut interval = tokio::time::interval(PUBLISHING_DELAY);

    loop {
        tokio::select! {
            Some(entry) = rx.recv() => {
                let key = (entry.source.clone(), entry.pair.clone());
                batched_data.insert(key, entry);
            },
            _ = interval.tick() => {
                let batch = std::mem::take(&mut batched_data);
                let batch = batch.into_values().collect();
                if let Err(e) = insert_funding_rate_entries(&pool, batch).await {
                    tracing::error!("❌ Failed to insert funding rates: {e:?}");
                }
            }
        }
    }
}

#[tracing::instrument(skip(pool, rx))]
pub async fn process_open_interest_entries(pool: Pool, mut rx: mpsc::Receiver<NewOpenInterest>) {
    let mut batched_data = HashMap::new();
    let mut interval = tokio::time::interval(PUBLISHING_DELAY);

    loop {
        tokio::select! {
            Some(entry) = rx.recv() => {
                let key = (entry.source.clone(), entry.pair.clone());
                batched_data.insert(key, entry);
            },
            _ = interval.tick() => {
                let batch = std::mem::take(&mut batched_data);
                let batch = batch.into_values().collect();
                if let Err(e) = insert_open_interest_entries(&pool, batch).await {
                    tracing::error!("❌ Failed to insert open interest entries: {e:?}");
                }
            }
        }
    }
}
