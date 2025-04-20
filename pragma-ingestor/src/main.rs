use dotenvy::{dotenv, var};
use futures_util::stream::StreamExt;
use rdkafka::Message as _;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tracing::{error, info};

use faucon_rs::config::{FauConfig, FauconEnvironment};
use faucon_rs::consumer::FauConsumer;
use faucon_rs::topics::FauconTopic;
use pragma_common::{
    CapnpDeserialize, InstrumentType,
    entries::{FundingRateEntry, PriceEntry},
    task_group::TaskGroup,
};
use pragma_entities::connection::ENV_OFFCHAIN_DATABASE_URL;
use pragma_entities::{NewEntry, NewFundingRate, NewFutureEntry};

use crate::config::CONFIG;
use crate::db::{process_funding_rate_entries, process_future_entries, process_spot_entries};

mod config;
mod db;
mod error;

const CHANNEL_CAPACITY: usize = 10_000;
const PUBLISHER_NAME: &str = "PRAGMA";
const KAFKA_GROUP_ID: &str = "pragma-ingestor";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize environment and telemetry
    dotenv().ok();

    let otel_endpoint = var("OTEL_EXPORTER_OTLP_ENDPOINT").ok();
    pragma_common::telemetry::init_telemetry("pragma-ingestor", otel_endpoint)?;

    // Load Kafka configuration
    let config = FauConfig::new(FauconEnvironment::Development);
    info!("Kafka configuration: hostname={:?}", config.broker_id);

    // Initialize database connection pool
    let pool = pragma_entities::connection::init_pool("pragma-ingestor", ENV_OFFCHAIN_DATABASE_URL)
        .expect("Failed to connect to offchain database");

    // Set up channels for spot, future, and funding rate entries with backpressure
    let (spot_tx, spot_rx) = mpsc::channel::<NewEntry>(CHANNEL_CAPACITY);
    let (future_tx, future_rx) = mpsc::channel::<NewFutureEntry>(CHANNEL_CAPACITY);
    let (funding_rate_tx, funding_rate_rx) = mpsc::channel::<NewFundingRate>(CHANNEL_CAPACITY);

    // Spawn database worker tasks
    let task_group = TaskGroup::new()
        .with_handle(tokio::spawn(process_spot_entries(pool.clone(), spot_rx)))
        .with_handle(tokio::spawn(process_future_entries(
            pool.clone(),
            future_rx,
        )))
        .with_handle(tokio::spawn(process_funding_rate_entries(
            pool,
            funding_rate_rx,
        )));

    // Spawn price consumers
    let mut join_set = JoinSet::new();
    for _ in 0..CONFIG.num_consumers {
        join_set.spawn(run_price_consumer(
            config.clone(),
            KAFKA_GROUP_ID.to_string(),
            spot_tx.clone(),
            future_tx.clone(),
        ));
        join_set.spawn(run_funding_rate_consumer(
            config.clone(),
            KAFKA_GROUP_ID.to_string(),
            funding_rate_tx.clone(),
        ));
    }

    while let Some(result) = join_set.join_next().await {
        if let Err(e) = result {
            error!("Consumer error: {}", e);
        }
    }

    // Drop original senders to close channels when consumers finish
    drop(spot_tx);
    drop(future_tx);
    drop(funding_rate_tx);

    // Await all tasks and abort if one fails
    task_group.abort_all_if_one_resolves().await;
    Ok(())
}

/// Runs a Kafka consumer for price entries
async fn run_price_consumer(
    config: FauConfig,
    group_id: String,
    spot_tx: mpsc::Sender<NewEntry>,
    future_tx: mpsc::Sender<NewFutureEntry>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut consumer = FauConsumer::new(config, &group_id)?;
    consumer.subscribe(FauconTopic::PRICES_V1)?;
    let mut stream = consumer.stream();

    tracing::info!("🚀 Starting price consumer");

    while let Some(msg_result) = stream.next().await {
        match msg_result {
            Ok(msg) => {
                let owned_message = msg.detach();
                if let Some(payload) = owned_message.payload() {
                    match PriceEntry::from_capnp(payload) {
                        Ok(entry) => {
                            let timestamp =
                                chrono::DateTime::from_timestamp_millis(entry.timestamp_ms)
                                    .map_or_else(
                                        || {
                                            error!("Invalid timestamp: {}", entry.timestamp_ms);
                                            chrono::NaiveDateTime::default()
                                        },
                                        |dt| dt.naive_utc(),
                                    );

                            match entry.instrument_type() {
                                InstrumentType::Spot => {
                                    let spot_entry = NewEntry {
                                        source: entry.source,
                                        pair_id: entry.pair.to_string(),
                                        publisher: PUBLISHER_NAME.to_string(),
                                        price: entry.price.into(),
                                        timestamp,
                                    };
                                    if let Err(e) = spot_tx.send(spot_entry).await {
                                        error!("Failed to send spot entry: {}", e);
                                    }
                                }
                                InstrumentType::Perp => {
                                    let future_entry = NewFutureEntry {
                                        pair_id: entry.pair.to_string(),
                                        publisher: PUBLISHER_NAME.to_string(),
                                        source: entry.source,
                                        price: entry.price.into(),
                                        timestamp,
                                        expiration_timestamp: None,
                                    };
                                    if let Err(e) = future_tx.send(future_entry).await {
                                        error!("Failed to send future entry: {}", e);
                                    }
                                }
                            }
                        }
                        Err(e) => error!("Failed to deserialize price entry: {}", e),
                    }
                }
            }
            Err(e) => error!("Consumer error: {}", e),
        }
    }

    Ok(())
}

/// Runs a Kafka consumer for funding rate entries
async fn run_funding_rate_consumer(
    config: FauConfig,
    group_id: String,
    funding_rate_tx: mpsc::Sender<NewFundingRate>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut consumer = FauConsumer::new(config, &group_id)?;
    consumer.subscribe(FauconTopic::FUNDING_RATES_V1)?;
    let mut stream = consumer.stream();

    tracing::info!("🚀 Starting funding rate consumer");

    while let Some(msg_result) = stream.next().await {
        match msg_result {
            Ok(msg) => {
                let owned_message = msg.detach();
                if let Some(payload) = owned_message.payload() {
                    match FundingRateEntry::from_capnp(payload) {
                        Ok(entry) => {
                            let funding_rate_entry = NewFundingRate {
                                source: entry.source,
                                pair: entry.pair.to_string(),
                                annualized_rate: entry.annualized_rate,
                                timestamp: chrono::DateTime::from_timestamp_millis(
                                    entry.timestamp_ms,
                                )
                                .map_or_else(
                                    || {
                                        error!("Invalid timestamp: {}", entry.timestamp_ms);
                                        chrono::NaiveDateTime::default()
                                    },
                                    |dt| dt.naive_utc(),
                                ),
                            };
                            if let Err(e) = funding_rate_tx.send(funding_rate_entry).await {
                                error!("Failed to send funding rate entry: {}", e);
                            }
                        }
                        Err(e) => error!("Failed to deserialize funding rate entry: {}", e),
                    }
                }
            }
            Err(e) => error!("Consumer error: {}", e),
        }
    }

    Ok(())
}
