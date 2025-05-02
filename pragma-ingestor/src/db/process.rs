use std::{collections::HashMap, time::Duration};

use deadpool_diesel::postgres::Pool;
use pragma_entities::{NewEntry, NewFundingRate, NewFutureEntry};
use tokio::{sync::mpsc, time::Instant};

use crate::db::insert::{insert_funding_rate_entries, insert_future_entries, insert_spot_entries};

#[tracing::instrument(skip(pool, rx))]
pub async fn process_spot_entries(pool: Pool, mut rx: mpsc::Receiver<NewEntry>) {
    let mut batched_prices = HashMap::new();

    let mut interval = tokio::time::interval_at(
        Instant::now() + Duration::from_secs(1),
        Duration::from_millis(100),
    );

    loop {
        tokio::select! {
            Some(entry) = rx.recv() => {
                let key = (entry.source.clone(), entry.pair_id.clone());
                batched_prices.insert(key, entry);
            },
            _ = interval.tick() => {
                let batch = std::mem::take(&mut batched_prices);
                let batch = batch.into_values().collect();
                if let Err(e) = insert_spot_entries(&pool, batch).await {
                    tracing::error!("❌ Failed to flush final spot entries: {}", e);
                }
            }
        }
    }
}

#[tracing::instrument(skip(pool, rx))]
pub async fn process_future_entries(pool: Pool, mut rx: mpsc::Receiver<NewFutureEntry>) {
    let mut batched_prices = HashMap::new();

    let mut interval = tokio::time::interval_at(
        Instant::now() + Duration::from_secs(1),
        Duration::from_millis(100),
    );

    loop {
        tokio::select! {
            Some(entry) = rx.recv() => {
                let key = (entry.source.clone(), entry.pair_id.clone());
                batched_prices.insert(key, entry);
            },
            _ = interval.tick() => {
                let batch = std::mem::take(&mut batched_prices);
                let batch = batch.into_values().collect();
                if let Err(e) = insert_future_entries(&pool, batch).await {
                    tracing::error!("❌ Failed to flush final spot entries: {}", e);
                }
            }
        }
    }
}

#[tracing::instrument(skip(pool, rx))]
pub async fn process_funding_rate_entries(pool: Pool, mut rx: mpsc::Receiver<NewFundingRate>) {
    let mut batched_prices = HashMap::new();

    let mut interval = tokio::time::interval_at(
        Instant::now() + Duration::from_secs(1),
        Duration::from_millis(100),
    );

    loop {
        tokio::select! {
            Some(entry) = rx.recv() => {
                let key = (entry.source.clone(), entry.pair.clone());
                batched_prices.insert(key, entry);
            },
            _ = interval.tick() => {
                let batch = std::mem::take(&mut batched_prices);
                let batch = batch.into_values().collect();
                if let Err(e) = insert_funding_rate_entries(&pool, batch).await {
                    tracing::error!("❌ Failed to flush final funding rate entries: {}", e);
                }
            }
        }
    }
}
