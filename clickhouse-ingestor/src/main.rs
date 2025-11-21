use clickhouse::{Client, Row};
use dotenvy::dotenv;
use faucon_rs::consumer::FauConsumerBuilder;
use faucon_rs::topics::prices::PriceFilter;
use faucon_rs::topics::FauconTopic;
use faucon_rs::{consumer::AutoOffsetReset, environment::FauconEnvironment};
use faucon_rs::{FauconEntry, FauconFilter as _};
use futures_util::StreamExt;
use pragma_common::task_group::TaskGroup;
use pragma_common::Pair;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info};
use uuid::Uuid;

mod config;

use crate::config::CONFIG;

/// Simple price entry for ClickHouse
#[derive(Debug, Clone, PartialEq, Row, Serialize, Deserialize)]
pub struct PriceEntry {
    #[serde(with = "clickhouse::serde::uuid")]
    pub id: Uuid,
    pub pair_id: String,
    pub price: String,
    pub timestamp: u32,
    pub source: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    // Initialize telemetry
    if let Some(ref endpoint) = CONFIG.otel_endpoint {
        pragma_common::telemetry::init_telemetry("clickhouse-ingestor", Some(endpoint.clone()))
            .expect("Failed to initialize telemetry");
        info!("Telemetry initialized with endpoint: {}", endpoint);
    }

    // Initialize ClickHouse client
    let client = Client::default()
        .with_url(&CONFIG.clickhouse_url)
        .with_database(&CONFIG.clickhouse_database);

    info!(
        "Connected to ClickHouse at {} (db: {})",
        CONFIG.clickhouse_url, CONFIG.clickhouse_database
    );

    // Set up channel for price entries
    let (tx, rx) = mpsc::channel::<PriceEntry>(CONFIG.channel_capacity);

    // Create task group
    let task_group = TaskGroup::new()
        .with_handle(tokio::spawn(process_entries(client, rx)))
        .with_handle(tokio::spawn(async move {
            if let Err(e) = run_price_consumer(tx).await {
                error!("Price consumer error: {}", e);
            }
        }));

    // Await all tasks
    task_group.abort_all_if_one_resolves().await;

    opentelemetry::global::shutdown_tracer_provider();

    Ok(())
}

/// Processes and batches entries before inserting into ClickHouse
async fn process_entries(client: Client, mut rx: mpsc::Receiver<PriceEntry>) {
    let mut batched_data: HashMap<(String, String), PriceEntry> = HashMap::new();
    let mut interval = tokio::time::interval(Duration::from_millis(CONFIG.flush_interval_ms));

    loop {
        tokio::select! {
            Some(entry) = rx.recv() => {
                let key = (entry.source.clone(), entry.pair_id.clone());
                batched_data.insert(key, entry);
            },
            _ = interval.tick() => {
                if batched_data.is_empty() {
                    continue;
                }

                let batch: Vec<PriceEntry> = std::mem::take(&mut batched_data).into_values().collect();
                let batch_size = batch.len();

                match insert_batch(&client, batch).await {
                    Ok(()) => {
                        info!("Inserted {} entries into ClickHouse", batch_size);
                    }
                    Err(e) => {
                        error!("Failed to insert entries: {}", e);
                    }
                }
            }
        }
    }
}

/// Inserts a batch of entries into ClickHouse
async fn insert_batch(client: &Client, entries: Vec<PriceEntry>) -> anyhow::Result<()> {
    if entries.is_empty() {
        return Ok(());
    }

    let mut insert = client.insert::<PriceEntry>("prices").await?;

    for entry in entries {
        insert.write(&entry).await?;
    }

    insert.end().await?;
    Ok(())
}

/// Runs the Kafka consumer for price entries
async fn run_price_consumer(tx: mpsc::Sender<PriceEntry>) -> anyhow::Result<()> {
    let kafka_environment = FauconEnvironment::Custom(CONFIG.kafka_broker_id.clone());
    let mut consumer = FauConsumerBuilder::on_environment(kafka_environment)
        .group_id(&CONFIG.kafka_group_id)
        .fetch_min_bytes(100_000)
        .fetch_wait_max_ms(25)
        .session_timeout(6000)
        .max_poll_interval(30000)
        .auto_commit(true)
        .auto_commit_interval(1000)
        .max_partition_fetch_bytes(1_048_576)
        .auto_offset_reset(AutoOffsetReset::Latest)
        .build()?;

    consumer.subscribe(&[FauconTopic::PRICES_V1])?;

    info!("Starting price consumer with {} pairs", CONFIG.pairs.len());

    // Build filter from configured pairs
    let pair_filters: Vec<PriceFilter> = CONFIG
        .pairs
        .iter()
        .filter_map(|p| {
            p.parse::<Pair>()
                .ok()
                .map(PriceFilter::Pair)
        })
        .collect();

    let price_filter = PriceFilter::Any(vec![PriceFilter::Any(pair_filters)]);

    let mut stream = consumer.filtered_stream(vec![price_filter.boxed()]);

    loop {
        if let Some(result) = stream.next().await {
            match result {
                Ok(entry) => {
                    if let FauconEntry::Price(entry) = entry {
                        let timestamp = chrono::DateTime::from_timestamp_millis(entry.timestamp_ms)
                            .map(|dt| dt.timestamp() as u32)
                            .unwrap_or(0);

                        let price_entry = PriceEntry {
                            id: Uuid::new_v4(),
                            pair_id: entry.pair.to_string(),
                            price: entry.price.to_string(),
                            timestamp,
                            source: entry.source,
                        };

                        if let Err(e) = tx.send(price_entry).await {
                            error!("Failed to send price entry: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to consume price entry: {}", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clickhouse::test;

    async fn do_insert(client: &Client, entries: Vec<PriceEntry>) -> clickhouse::error::Result<()> {
        let mut insert = client.insert::<PriceEntry>("prices").await?;
        for entry in entries {
            insert.write(&entry).await?;
        }
        insert.end().await
    }

    #[tokio::test]
    async fn test_insert_batch() {
        let mock = test::Mock::new();
        let client = Client::default().with_mock(&mock);

        let entries = vec![
            PriceEntry {
                id: Uuid::new_v4(),
                pair_id: "BTC/USD".to_string(),
                price: "50000.00".to_string(),
                timestamp: 1700000000,
                source: "binance".to_string(),
            },
            PriceEntry {
                id: Uuid::new_v4(),
                pair_id: "ETH/USD".to_string(),
                price: "3000.00".to_string(),
                timestamp: 1700000000,
                source: "binance".to_string(),
            },
        ];

        // Record the insert
        let recording = mock.add(test::handlers::record());
        do_insert(&client, entries.clone()).await.unwrap();

        // Verify recorded rows match
        let rows: Vec<PriceEntry> = recording.collect().await;
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].pair_id, "BTC/USD");
        assert_eq!(rows[1].pair_id, "ETH/USD");
    }

    #[tokio::test]
    async fn test_insert_empty_batch() {
        let mock = test::Mock::new();
        let client = Client::default().with_mock(&mock);

        // Empty batch should succeed without calling ClickHouse
        let result = insert_batch(&client, vec![]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_insert_failure() {
        let mock = test::Mock::new();
        let client = Client::default().with_mock(&mock);

        let entries = vec![PriceEntry {
            id: Uuid::new_v4(),
            pair_id: "BTC/USD".to_string(),
            price: "50000.00".to_string(),
            timestamp: 1700000000,
            source: "binance".to_string(),
        }];

        // Simulate server error
        mock.add(test::handlers::failure(test::status::INTERNAL_SERVER_ERROR));
        let result = do_insert(&client, entries).await;
        assert!(result.is_err());
    }
}
