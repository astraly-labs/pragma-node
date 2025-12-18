use clickhouse::Client;
use dotenvy::dotenv;
use pragma_common::task_group::TaskGroup;
use tokio::sync::mpsc;
use tracing::{error, info};

mod config;
mod consumers;
mod entries;
mod insert;
mod processors;

use crate::config::CONFIG;
use crate::consumers::{
    run_funding_rate_consumer, run_open_interest_consumer, run_price_consumer, run_trade_consumer,
};
use crate::entries::{FundingRateEntry, OpenInterestEntry, PriceEntry, TradeEntry};
use crate::processors::{
    process_funding_rate_entries, process_open_interest_entries, process_price_entries,
    process_trade_entries,
};

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
        .with_password(&CONFIG.clickhouse_password)
        .with_user(&CONFIG.clickhouse_user)
        .with_url(&CONFIG.clickhouse_url)
        .with_database(&CONFIG.clickhouse_database);

    info!(
        "Connected to ClickHouse at {} (db: {})",
        CONFIG.clickhouse_url, CONFIG.clickhouse_database
    );

    // Set up channels for all entry types
    let (price_tx, price_rx) = mpsc::channel::<PriceEntry>(CONFIG.channel_capacity);
    let (funding_rate_tx, funding_rate_rx) =
        mpsc::channel::<FundingRateEntry>(CONFIG.channel_capacity);
    let (open_interest_tx, open_interest_rx) =
        mpsc::channel::<OpenInterestEntry>(CONFIG.channel_capacity);
    let (trade_tx, trade_rx) = mpsc::channel::<TradeEntry>(CONFIG.channel_capacity);

    // Create task group
    let task_group = TaskGroup::new()
        .with_handle(tokio::spawn(process_price_entries(
            client.clone(),
            price_rx,
        )))
        .with_handle(tokio::spawn(process_funding_rate_entries(
            client.clone(),
            funding_rate_rx,
        )))
        .with_handle(tokio::spawn(process_open_interest_entries(
            client.clone(),
            open_interest_rx,
        )))
        .with_handle(tokio::spawn(process_trade_entries(
            client.clone(),
            trade_rx,
        )))
        .with_handle(tokio::spawn(async move {
            if let Err(e) = run_price_consumer(price_tx).await {
                error!("Price consumer error: {}", e);
            }
        }))
        .with_handle(tokio::spawn(async move {
            if let Err(e) = run_funding_rate_consumer(funding_rate_tx).await {
                error!("Funding rate consumer error: {}", e);
            }
        }))
        .with_handle(tokio::spawn(async move {
            if let Err(e) = run_open_interest_consumer(open_interest_tx).await {
                error!("Open interest consumer error: {}", e);
            }
        }))
        .with_handle(tokio::spawn(async move {
            if let Err(e) = run_trade_consumer(trade_tx).await {
                error!("Trade consumer error: {}", e);
            }
        }));

    // Await all tasks
    task_group.abort_all_if_one_resolves().await;

    opentelemetry::global::shutdown_tracer_provider();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clickhouse::test;
    use uuid::Uuid;

    use crate::entries::PriceEntry;
    use crate::insert::insert_price_batch;

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
        let result = insert_price_batch(&client, vec![]).await;
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
