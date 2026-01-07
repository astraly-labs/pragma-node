use clickhouse::Client;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::config::CONFIG;
use crate::entries::{FundingRateEntry, OpenInterestEntry, PriceEntry, TradeEntry};
use crate::insert::{
    insert_funding_rate_batch, insert_open_interest_batch, insert_price_batch, insert_trade_batch,
};

/// Processes and batches price entries before inserting into ClickHouse
pub(crate) async fn process_price_entries(client: Client, mut rx: mpsc::Receiver<PriceEntry>) {
    let mut batched_data: HashMap<(String, String), PriceEntry> = HashMap::new();
    let mut interval = tokio::time::interval(Duration::from_millis(CONFIG.flush_interval_ms));

    loop {
        tokio::select! {
            Some(entry) = rx.recv() => {
                // Use market_id for deduplication (includes instrument type)
                let key = (entry.source.clone(), entry.market_id.clone());
                batched_data.insert(key, entry);
            },
            _ = interval.tick() => {
                if batched_data.is_empty() {
                    continue;
                }

                let batch: Vec<PriceEntry> = std::mem::take(&mut batched_data).into_values().collect();
                let batch_size = batch.len();

                match insert_price_batch(&client, batch).await {
                    Ok(()) => {
                        info!("Inserted {} price entries into ClickHouse", batch_size);
                    }
                    Err(e) => {
                        error!("Failed to insert price entries: {}", e);
                    }
                }
            }
        }
    }
}

/// Processes and batches funding rate entries before inserting into ClickHouse
pub(crate) async fn process_funding_rate_entries(
    client: Client,
    mut rx: mpsc::Receiver<FundingRateEntry>,
) {
    let mut batched_data: HashMap<(String, String), FundingRateEntry> = HashMap::new();
    let mut interval = tokio::time::interval(Duration::from_millis(CONFIG.flush_interval_ms));

    loop {
        tokio::select! {
            Some(entry) = rx.recv() => {
                // Use market_id for deduplication (includes instrument type)
                let key = (entry.source.clone(), entry.market_id.clone());
                batched_data.insert(key, entry);
            },
            _ = interval.tick() => {
                if batched_data.is_empty() {
                    continue;
                }

                let batch: Vec<FundingRateEntry> = std::mem::take(&mut batched_data).into_values().collect();
                let batch_size = batch.len();

                match insert_funding_rate_batch(&client, batch).await {
                    Ok(()) => {
                        info!("Inserted {} funding rate entries into ClickHouse", batch_size);
                    }
                    Err(e) => {
                        error!("Failed to insert funding rate entries: {}", e);
                    }
                }
            }
        }
    }
}

/// Processes and batches open interest entries before inserting into ClickHouse
pub(crate) async fn process_open_interest_entries(
    client: Client,
    mut rx: mpsc::Receiver<OpenInterestEntry>,
) {
    let mut batched_data: HashMap<(String, String), OpenInterestEntry> = HashMap::new();
    let mut interval = tokio::time::interval(Duration::from_millis(CONFIG.flush_interval_ms));

    loop {
        tokio::select! {
            Some(entry) = rx.recv() => {
                // Use market_id for deduplication (includes instrument type)
                let key = (entry.source.clone(), entry.market_id.clone());
                batched_data.insert(key, entry);
            },
            _ = interval.tick() => {
                if batched_data.is_empty() {
                    continue;
                }

                let batch: Vec<OpenInterestEntry> = std::mem::take(&mut batched_data).into_values().collect();
                let batch_size = batch.len();

                match insert_open_interest_batch(&client, batch).await {
                    Ok(()) => {
                        info!("Inserted {} open interest entries into ClickHouse", batch_size);
                    }
                    Err(e) => {
                        error!("Failed to insert open interest entries: {}", e);
                    }
                }
            }
        }
    }
}

/// Processes and batches trade entries before inserting into ClickHouse
pub(crate) async fn process_trade_entries(client: Client, mut rx: mpsc::Receiver<TradeEntry>) {
    let mut batched_data: Vec<TradeEntry> = Vec::new();
    let mut interval = tokio::time::interval(Duration::from_millis(CONFIG.flush_interval_ms));

    loop {
        tokio::select! {
            Some(entry) = rx.recv() => {
                batched_data.push(entry);
            },
            _ = interval.tick() => {
                if batched_data.is_empty() {
                    continue;
                }

                let batch = std::mem::take(&mut batched_data);
                let batch_size = batch.len();

                match insert_trade_batch(&client, batch).await {
                    Ok(()) => {
                        info!("Inserted {} trade entries into ClickHouse", batch_size);
                    }
                    Err(e) => {
                        error!("Failed to insert trade entries: {}", e);
                    }
                }
            }
        }
    }
}
