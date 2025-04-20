use dotenvy::{dotenv, var};
use futures_util::stream::{FuturesUnordered, StreamExt};
use rdkafka::Message as _;
use rdkafka::message::OwnedMessage;
use tokio::sync::mpsc;
use tracing::{error, info};

use faucon_rs::config::{FauConfig, FauconEnvironment};
use faucon_rs::consumer::FauConsumer;
use faucon_rs::topics::FauconTopic;
use pragma_common::{CapnpDeserialize, InstrumentType, entries::PriceEntry, task_group::TaskGroup};
use pragma_entities::connection::ENV_OFFCHAIN_DATABASE_URL;
use pragma_entities::{NewEntry, NewFutureEntry};

use crate::config::CONFIG;
use crate::db::{process_future_entries, process_spot_entries};

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

    // Set up channels for spot and future entries with backpressure
    let (spot_tx, spot_rx) = mpsc::channel::<NewEntry>(CHANNEL_CAPACITY);
    let (future_tx, future_rx) = mpsc::channel::<NewFutureEntry>(CHANNEL_CAPACITY);

    // Spawn database worker tasks
    let task_group = TaskGroup::new()
        .with_handle(tokio::spawn(process_spot_entries(pool.clone(), spot_rx)))
        .with_handle(tokio::spawn(process_future_entries(pool, future_rx)));

    (0..CONFIG.num_consumers)
        .map(|_| {
            tokio::spawn(run_consumer(
                config.clone(),
                KAFKA_GROUP_ID.to_string(),
                spot_tx.clone(),
                future_tx.clone(),
            ))
        })
        .collect::<FuturesUnordered<_>>()
        .for_each(|_| async { () })
        .await;

    // Drop original senders to close channels when consumers finish
    drop(spot_tx);
    drop(future_tx);

    // Await all tasks and abort if one fails
    task_group.abort_all_if_one_resolves().await;
    Ok(())
}

/// Runs a single Kafka consumer, processing messages and sending entries to channels.
async fn run_consumer(
    config: FauConfig,
    group_id: String,
    spot_tx: mpsc::Sender<NewEntry>,
    future_tx: mpsc::Sender<NewFutureEntry>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize and subscribe Kafka consumer
    let mut consumer = FauConsumer::new(config, &group_id)?;
    consumer.subscribe(FauconTopic::PRICES_V1)?;
    let mut stream = consumer.stream();

    // Process messages from the Kafka stream
    while let Some(msg_result) = stream.next().await {
        match msg_result {
            Ok(msg) => {
                let owned_message = msg.detach();
                process_message(&owned_message, &spot_tx, &future_tx).await;
            }
            Err(e) => error!("Consumer error: {}", e),
        }
    }

    Ok(())
}

/// Processes a single Kafka message and routes it to the appropriate channel.
async fn process_message(
    msg: &OwnedMessage,
    spot_tx: &mpsc::Sender<NewEntry>,
    future_tx: &mpsc::Sender<NewFutureEntry>,
) {
    if let Some(payload) = msg.payload() {
        match PriceEntry::from_capnp(payload) {
            Ok(entry) => {
                // Convert timestamp to NaiveDateTime
                let timestamp = chrono::DateTime::from_timestamp_millis(entry.timestamp_ms)
                    .map(|dt| dt.naive_utc())
                    .unwrap_or_else(|| {
                        error!("Invalid timestamp: {}", entry.timestamp_ms);
                        chrono::NaiveDateTime::default()
                    });

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
    } else {
        error!("Received message with no payload");
    }
}
