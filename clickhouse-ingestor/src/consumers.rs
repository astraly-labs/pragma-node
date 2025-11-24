use faucon_rs::consumer::FauConsumerBuilder;
use faucon_rs::topics::FauconTopic;
use faucon_rs::topics::prices::PriceFilter;
use faucon_rs::{FauconEntry, FauconFilter as _};
use faucon_rs::{consumer::AutoOffsetReset, environment::FauconEnvironment};
use futures_util::StreamExt;
use pragma_common::Pair;
use tokio::sync::mpsc;
use tracing::{error, info};
use uuid::Uuid;

use crate::config::CONFIG;
use crate::entries::{FundingRateEntry, OpenInterestEntry, PriceEntry, TradeEntry};

/// Runs the Kafka consumer for price entries
pub(crate) async fn run_price_consumer(tx: mpsc::Sender<PriceEntry>) -> anyhow::Result<()> {
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
        .filter_map(|p| p.parse::<Pair>().ok().map(PriceFilter::Pair))
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

/// Runs the Kafka consumer for funding rate entries
pub(crate) async fn run_funding_rate_consumer(tx: mpsc::Sender<FundingRateEntry>) -> anyhow::Result<()> {
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

    consumer.subscribe(&[FauconTopic::FUNDING_RATES_V1])?;

    info!("Starting funding rate consumer");

    let mut stream = consumer.stream();

    loop {
        if let Some(result) = stream.next().await {
            match result {
                Ok(entry) => {
                    if let FauconEntry::FundingRate(entry) = entry {
                        let timestamp = chrono::DateTime::from_timestamp_millis(entry.timestamp_ms)
                            .map(|dt| dt.timestamp() as u32)
                            .unwrap_or(0);

                        let funding_rate_entry = FundingRateEntry {
                            id: Uuid::new_v4(),
                            pair_id: entry.pair.to_string(),
                            annualized_rate: entry.annualized_rate,
                            timestamp,
                            source: entry.source,
                        };

                        if let Err(e) = tx.send(funding_rate_entry).await {
                            error!("Failed to send funding rate entry: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to consume funding rate entry: {}", e);
                }
            }
        }
    }
}

/// Runs the Kafka consumer for open interest entries
pub(crate) async fn run_open_interest_consumer(tx: mpsc::Sender<OpenInterestEntry>) -> anyhow::Result<()> {
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

    consumer.subscribe(&[FauconTopic::OPEN_INTEREST_V1])?;

    info!("Starting open interest consumer");

    let mut stream = consumer.stream();

    loop {
        if let Some(result) = stream.next().await {
            match result {
                Ok(entry) => {
                    if let FauconEntry::OpenInterest(entry) = entry {
                        let timestamp = chrono::DateTime::from_timestamp_millis(entry.timestamp_ms)
                            .map(|dt| dt.timestamp() as u32)
                            .unwrap_or(0);

                        let open_interest_entry = OpenInterestEntry {
                            id: Uuid::new_v4(),
                            pair_id: entry.pair.to_string(),
                            open_interest_value: entry.open_interest,
                            timestamp,
                            source: entry.source,
                        };

                        if let Err(e) = tx.send(open_interest_entry).await {
                            error!("Failed to send open interest entry: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to consume open interest entry: {}", e);
                }
            }
        }
    }
}

/// Runs the Kafka consumer for trade entries
pub(crate) async fn run_trade_consumer(tx: mpsc::Sender<TradeEntry>) -> anyhow::Result<()> {
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

    consumer.subscribe(&[FauconTopic::TRADES_V1])?;

    info!("Starting trade consumer");

    let mut stream = consumer.stream();

    loop {
        if let Some(result) = stream.next().await {
            match result {
                Ok(entry) => {
                    if let FauconEntry::Trade(entry) = entry {
                        let timestamp = chrono::DateTime::from_timestamp_millis(entry.timestamp_ms)
                            .map(|dt| dt.timestamp() as u32)
                            .unwrap_or(0);

                        let side_str = format!("{:?}", entry.side);
                        let trade_entry = TradeEntry {
                            id: Uuid::new_v4(),
                            pair_id: entry.pair.to_string(),
                            price: entry.price.to_string(),
                            size: entry.size.to_string(),
                            side: side_str,
                            timestamp,
                            source: entry.source,
                        };

                        if let Err(e) = tx.send(trade_entry).await {
                            error!("Failed to send trade entry: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to consume trade entry: {}", e);
                }
            }
        }
    }
}

