use deadpool_diesel::postgres::Pool;
use pragma_entities::{NewEntry, NewFundingRate, NewFutureEntry};
use tokio::sync::mpsc;

use crate::db::insert::{insert_funding_rate_entries, insert_future_entries, insert_spot_entries};

const BATCH_SIZE: usize = 100;

#[tracing::instrument(skip(pool, rx))]
pub async fn process_spot_entries(pool: Pool, mut rx: mpsc::Receiver<NewEntry>) {
    let mut buffer = Vec::with_capacity(BATCH_SIZE);

    loop {
        tokio::select! {
            Some(entry) = rx.recv() => {
                buffer.push(entry);

                if buffer.len() >= BATCH_SIZE {
                    if let Err(e) = insert_spot_entries(&pool, std::mem::take(&mut buffer)).await {
                        tracing::error!("❌ Failed to insert spot entries: {}", e);
                    }
                    buffer = Vec::with_capacity(BATCH_SIZE);
                }
            }
            else => {
                // Channel closed, flush remaining entries
                if !buffer.is_empty() {
                    if let Err(e) = insert_spot_entries(&pool, buffer).await {
                        tracing::error!("❌ Failed to flush final spot entries: {}", e);
                    }
                }
                break;
            }
        }
    }
}

#[tracing::instrument(skip(pool, rx))]
pub async fn process_future_entries(pool: Pool, mut rx: mpsc::Receiver<NewFutureEntry>) {
    let mut buffer = Vec::with_capacity(BATCH_SIZE);

    loop {
        tokio::select! {
            Some(entry) = rx.recv() => {
                buffer.push(entry);

                if buffer.len() >= BATCH_SIZE {
                    if let Err(e) = insert_future_entries(&pool, std::mem::take(&mut buffer)).await {
                        tracing::error!("❌ Failed to insert future entries: {}", e);
                    }
                    buffer = Vec::with_capacity(BATCH_SIZE);
                }
            }
            else => {
                // Channel closed, flush remaining entries
                if !buffer.is_empty() {
                    if let Err(e) = insert_future_entries(&pool, buffer).await {
                        tracing::error!("❌ Failed to flush final future entries: {}", e);
                    }
                }
                break;
            }
        }
    }
}

#[tracing::instrument(skip(pool, rx))]
pub async fn process_funding_rate_entries(pool: Pool, mut rx: mpsc::Receiver<NewFundingRate>) {
    let mut buffer = Vec::with_capacity(BATCH_SIZE);

    loop {
        tokio::select! {
            Some(entry) = rx.recv() => {
                buffer.push(entry);

                if buffer.len() >= BATCH_SIZE {
                    if let Err(e) = insert_funding_rate_entries(&pool, std::mem::take(&mut buffer)).await {
                        tracing::error!("❌ Failed to insert funding rate entries: {}", e);
                    }
                    buffer = Vec::with_capacity(BATCH_SIZE);
                }
            }
            else => {
                // Channel closed, flush remaining entries
                if !buffer.is_empty() {
                    if let Err(e) = insert_funding_rate_entries(&pool, buffer).await {
                        tracing::error!("❌ Failed to flush final funding rate entries: {}", e);
                    }
                }
                break;
            }
        }
    }
}
