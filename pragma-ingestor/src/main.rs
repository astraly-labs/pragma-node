pub mod config;
mod error;

use deadpool_diesel::postgres::Pool;
use dotenvy::dotenv;
use faucon_rs::Message as _;
use faucon_rs::config::{FauConfig, FauconEnvironment};
use faucon_rs::consumer::FauConsumer;
use faucon_rs::topics::FauconTopic;
use futures_util::stream::StreamExt as _;
use pragma_common::InstrumentType;
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};
use tracing::{debug, info};

use pragma_common::{CapnpDeserialize, entries::PriceEntry};

use pragma_entities::connection::ENV_OFFCHAIN_DATABASE_URL;
use pragma_entities::{Entry, FutureEntry, InfraError, NewEntry, NewFutureEntry};

#[tokio::main]
#[tracing::instrument]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv()?;

    let otel_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok();
    pragma_common::telemetry::init_telemetry("pragma-ingestor".into(), otel_endpoint)?;

    let faucon_config = FauConfig::new(FauconEnvironment::Development);

    info!(
        "kafka configuration : hostname={:?}",
        faucon_config.broker_id
    );

    let mut consumer = FauConsumer::new(faucon_config, "pragma_ingestor")?;

    let pool = pragma_entities::connection::init_pool("pragma-ingestor", ENV_OFFCHAIN_DATABASE_URL)
        .expect("cannot connect to offchain database");

    consumer.subscribe(FauconTopic::PRICES_V1)?;

    // Create channels for spot and future entries
    const CHANNEL_CAPACITY: usize = 1000;
    let (spot_tx, spot_rx) = mpsc::channel::<NewEntry>(CHANNEL_CAPACITY);
    let (future_tx, future_rx) = mpsc::channel::<NewFutureEntry>(CHANNEL_CAPACITY);

    // Spawn database worker tasks
    let spot_pool = pool.clone();
    tokio::spawn(process_spot_entries(spot_pool, spot_rx));
    tokio::spawn(process_future_entries(pool, future_rx));

    let mut stream = consumer.stream();

    while let Some(msg_result) = stream.next().await {
        match msg_result {
            Ok(msg) => match msg.payload() {
                Some(payload) => match PriceEntry::from_capnp(payload) {
                    Ok(entry) => {
                        let ts = chrono::DateTime::from_timestamp_millis(entry.timestamp_ms)
                            .map(|dt| dt.naive_utc())
                            .unwrap();
                        let source = entry.clone().source;
                        let instrument_type = entry.instrument_type();

                        match instrument_type {
                            InstrumentType::Spot => {
                                let new_entry = NewEntry {
                                    source,
                                    pair_id: entry.pair.to_string(),
                                    publisher: "PRAGMA".to_string(),
                                    price: entry.price.into(),
                                    timestamp: ts,
                                };
                                if let Err(e) = spot_tx.send(new_entry).await {
                                    tracing::error!("Failed to send spot entry to channel: {}", e);
                                }
                            }
                            InstrumentType::Perp => {
                                let new_entry = NewFutureEntry {
                                    pair_id: entry.pair.to_string(),
                                    publisher: "PRAGMA".to_string(),
                                    source,
                                    price: entry.price.into(),
                                    timestamp: ts,
                                    expiration_timestamp: None,
                                };
                                if let Err(e) = future_tx.send(new_entry).await {
                                    tracing::error!(
                                        "Failed to send future entry to channel: {}",
                                        e
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to deserialize price entry: {}", e);
                        continue;
                    }
                },
                None => {
                    tracing::warn!("Received message with no payload");
                    continue;
                }
            },
            Err(e) => {
                tracing::error!("Consumer error: {}", e);
                continue;
            }
        }
    }

    Ok(())
}

#[tracing::instrument(skip(pool, rx))]
async fn process_spot_entries(pool: Pool, mut rx: mpsc::Receiver<NewEntry>) {
    const BUFFER_CAPACITY: usize = 100;
    const FLUSH_TIMEOUT: Duration = Duration::from_millis(100);

    let mut buffer = Vec::with_capacity(BUFFER_CAPACITY);
    let mut flush_interval = interval(FLUSH_TIMEOUT);

    loop {
        tokio::select! {
            Some(entry) = rx.recv() => {
                buffer.push(entry);

                if buffer.len() >= BUFFER_CAPACITY {
                    if let Err(e) = insert_spot_entries(&pool, std::mem::take(&mut buffer)).await {
                        tracing::error!("Failed to insert spot entries: {}", e);
                    }
                    buffer = Vec::with_capacity(BUFFER_CAPACITY);
                }
            }
            _ = flush_interval.tick() => {
                if !buffer.is_empty() {
                    if let Err(e) = insert_spot_entries(&pool, std::mem::take(&mut buffer)).await {
                        tracing::error!("Failed to flush spot entries: {}", e);
                    }
                    buffer = Vec::with_capacity(BUFFER_CAPACITY);
                }
            }
            else => {
                // Channel closed, flush remaining entries
                if !buffer.is_empty() {
                    if let Err(e) = insert_spot_entries(&pool, buffer).await {
                        tracing::error!("Failed to flush final spot entries: {}", e);
                    }
                }
                break;
            }
        }
    }
}

#[tracing::instrument(skip(pool, rx))]
async fn process_future_entries(pool: Pool, mut rx: mpsc::Receiver<NewFutureEntry>) {
    const BUFFER_CAPACITY: usize = 100;
    const FLUSH_TIMEOUT: Duration = Duration::from_secs(30);

    let mut buffer = Vec::with_capacity(BUFFER_CAPACITY);
    let mut flush_interval = interval(FLUSH_TIMEOUT);

    loop {
        tokio::select! {
            Some(entry) = rx.recv() => {
                buffer.push(entry);

                if buffer.len() >= BUFFER_CAPACITY {
                    if let Err(e) = insert_future_entries(&pool, std::mem::take(&mut buffer)).await {
                        tracing::error!("Failed to insert future entries: {}", e);
                    }
                    buffer = Vec::with_capacity(BUFFER_CAPACITY);
                }
            }
            _ = flush_interval.tick() => {
                if !buffer.is_empty() {
                    if let Err(e) = insert_future_entries(&pool, std::mem::take(&mut buffer)).await {
                        tracing::error!("Failed to flush future entries: {}", e);
                    }
                    buffer = Vec::with_capacity(BUFFER_CAPACITY);
                }
            }
            else => {
                // Channel closed, flush remaining entries
                if !buffer.is_empty() {
                    if let Err(e) = insert_future_entries(&pool, buffer).await {
                        tracing::error!("Failed to flush final future entries: {}", e);
                    }
                }
                break;
            }
        }
    }
}

#[tracing::instrument(skip(pool))]
pub async fn insert_spot_entries(
    pool: &Pool,
    new_entries: Vec<NewEntry>,
) -> Result<(), InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;
    let entries = conn
        .interact(move |conn| Entry::create_many(conn, new_entries))
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    for entry in &entries {
        debug!(
            "new entry created {} - {}({}) - {}",
            entry.publisher, entry.pair_id, entry.price, entry.source
        );
    }

    Ok(())
}

#[tracing::instrument(skip(pool))]
pub async fn insert_future_entries(
    pool: &Pool,
    new_entries: Vec<NewFutureEntry>,
) -> Result<(), InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;

    let new_entries = new_entries
        .into_iter()
        .map(|mut entry| {
            if let Some(expiration_timestamp) = entry.expiration_timestamp {
                if expiration_timestamp.and_utc().timestamp() == 0 {
                    entry.expiration_timestamp = None;
                }
            }
            entry
        })
        .collect::<Vec<_>>();

    let len_perp_entries = new_entries
        .iter()
        .filter(|entry| entry.expiration_timestamp.is_none())
        .count();

    debug!("[PERP] {} new entries available", len_perp_entries);
    debug!(
        "[FUTURE] {} new entries available",
        new_entries.len() - len_perp_entries
    );

    let entries = conn
        .interact(move |conn| FutureEntry::create_many(conn, new_entries))
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;
    for entry in &entries {
        debug!(
            "new future entry created {} - {}({}) - {}",
            entry.publisher, entry.pair_id, entry.price, entry.source
        );
    }
    Ok(())
}
