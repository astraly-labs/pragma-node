use dotenvy::dotenv;
use faucon_rs::consumer::FauConsumerBuilder;
use faucon_rs::topics::FauconTopic;
use faucon_rs::{consumer::AutoOffsetReset, environment::FauconEnvironment};
use futures_util::StreamExt;
use pragma_common::{
    InstrumentType,
    entries::{FundingRateEntry, PriceEntry, open_interest::OpenInterestEntry},
    task_group::TaskGroup,
};
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio::time::{interval, Duration};
use tracing::{error, warn};
use std::sync::Arc;

use pragma_entities::connection::ENV_OFFCHAIN_DATABASE_URL;
use pragma_entities::{NewEntry, NewFundingRate, NewFutureEntry, NewOpenInterest};

use crate::config::CONFIG;
use crate::db::process::{
    process_funding_rate_entries, process_future_entries, process_open_interest_entries,
    process_spot_entries,
};
use crate::metrics::{ConsumerType, IngestorMetricsRegistry};

mod config;
mod db;
mod error;
mod metrics;

#[tokio::main]
#[tracing::instrument]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    // We export our telemetry - so we can monitor the ingestor through Grafana.
    pragma_common::telemetry::init_telemetry("pragma-ingestor", CONFIG.otel_endpoint.clone())
        .expect("Failed to initialize telemetry");

    let pool = pragma_entities::connection::init_pool("pragma-ingestor", ENV_OFFCHAIN_DATABASE_URL)
        .expect("Failed to connect to offchain database");

    // Initialize metrics registry
    let metrics_registry = metrics::IngestorMetricsRegistry::new();

    // Start data freshness monitoring task
    let freshness_pool = pool.clone();
    let freshness_metrics = metrics_registry.clone();
    tokio::spawn(metrics::start_data_freshness_monitor(
        freshness_pool,
        freshness_metrics,
    ));

    // Set up channels for spot, future, and funding rate entries with backpressure
    let (spot_tx, spot_rx) = mpsc::channel::<NewEntry>(CONFIG.channel_capacity * 2);
    let (future_tx, future_rx) = mpsc::channel::<NewFutureEntry>(CONFIG.channel_capacity);
    let (funding_rate_tx, funding_rate_rx) =
        mpsc::channel::<NewFundingRate>(CONFIG.channel_capacity / 2);
    let (open_interest_tx, open_interest_rx) =
        mpsc::channel::<NewOpenInterest>(CONFIG.channel_capacity / 2);

    // Spawn database worker tasks
    let task_group = TaskGroup::new()
        .with_handle(tokio::spawn(process_spot_entries(
            pool.clone(),
            spot_rx,
            metrics_registry.clone(),
        )))
        .with_handle(tokio::spawn(process_future_entries(
            pool.clone(),
            future_rx,
            metrics_registry.clone(),
        )))
        .with_handle(tokio::spawn(process_funding_rate_entries(
            pool.clone(),
            funding_rate_rx,
            metrics_registry.clone(),
        )))
        .with_handle(tokio::spawn(process_open_interest_entries(
            pool,
            open_interest_rx,
            metrics_registry.clone(),
        )));

    // Spawn consumers
    let mut join_set = JoinSet::new();
    for _ in 0..CONFIG.num_consumers {
        join_set.spawn(run_price_consumer(
            CONFIG.kafka_group_id.clone(),
            spot_tx.clone(),
            future_tx.clone(),
            metrics_registry.clone(),
        ));
        join_set.spawn(run_funding_rate_consumer(
            CONFIG.kafka_group_id.clone(),
            funding_rate_tx.clone(),
            metrics_registry.clone(),
        ));
        join_set.spawn(run_open_interest_consumer(
            CONFIG.kafka_group_id.clone(),
            open_interest_tx.clone(),
            metrics_registry.clone(),
        ));
    }

    while let Some(result) = join_set.join_next().await {
        if let Err(e) = result {
            error!("Consumer error: {e}");
        }
    }

    // Drop original senders to close channels when consumers finish
    drop(spot_tx);
    drop(future_tx);
    drop(funding_rate_tx);
    drop(open_interest_tx);

    // Await all tasks and abort if one fails
    task_group.abort_all_if_one_resolves().await;
    Ok(())
}

/// Runs a Kafka consumer for price entries
#[tracing::instrument(skip_all)]
async fn run_price_consumer(
    group_id: String,
    spot_tx: mpsc::Sender<NewEntry>,
    future_tx: mpsc::Sender<NewFutureEntry>,
    metrics_registry: Arc<IngestorMetricsRegistry>,
) -> anyhow::Result<()> {
    let kafka_environment = FauconEnvironment::Custom(CONFIG.kafka_broker_id.clone());
    let mut consumer = FauConsumerBuilder::on_environment(kafka_environment)
        .group_id(&group_id)
        .fetch_min_bytes(100_000)
        .fetch_wait_max_ms(25)
        .session_timeout(6000)
        .max_poll_interval(30000)
        .auto_commit(true)
        .auto_commit_interval(1000)
        .max_partition_fetch_bytes(1_048_576)
        .auto_offset_reset(faucon_rs::consumer::AutoOffsetReset::Latest)
        .build()?;

    consumer.subscribe(&[FauconTopic::PRICES_V1])?;

    tracing::info!("🚀 Starting price consumer");

    let mut stream = consumer.stream();
    let mut check_lag_interval = interval(Duration::from_secs(2));

    loop {
        tokio::select! {
            Some(result) = stream.next() => {
                match result {
                    Ok(entry) => {
                        if let Ok(entry) = entry.try_into::<PriceEntry>() {
                            let timestamp = chrono::DateTime::from_timestamp_millis(entry.timestamp_ms)
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
                                        publisher: CONFIG.publisher_name.clone(),
                                        price: entry.price.into(),
                                        timestamp,
                                    };
                                    if let Err(e) = spot_tx.send(spot_entry).await {
                                        error!("Failed to send spot entry: {e}");
                                    }
                                }
                                InstrumentType::Perp => {
                                    let future_entry = NewFutureEntry {
                                        pair_id: entry.pair.to_string(),
                                        publisher: CONFIG.publisher_name.clone(),
                                        source: entry.source,
                                        price: entry.price.into(),
                                        timestamp,
                                        expiration_timestamp: None,
                                    };
                                    if let Err(e) = future_tx.send(future_entry).await {
                                        error!("Failed to send future entry: {e}");
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to consume price entry: {e}");
                    }
                }
            }

            _ = check_lag_interval.tick() => {
                match consumer.lag() {
                    Ok(lag) => {
                        if !lag.is_empty() {
                            let total_lag: i64 = lag.iter().map(|(_, lag)| *lag).sum();
                            metrics_registry.record_consumer_lag(ConsumerType::Price, total_lag);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to get price consumer lag: {e:?}");
                    }
                }
            }
        }
    }

    Ok(())
}

/// Runs a Kafka consumer for funding rate entries
#[tracing::instrument(skip_all)]
async fn run_funding_rate_consumer(
    group_id: String,
    funding_rate_tx: mpsc::Sender<NewFundingRate>,
    metrics_registry: Arc<IngestorMetricsRegistry>,
) -> anyhow::Result<()> {
    let kafka_environment = FauconEnvironment::Custom(CONFIG.kafka_broker_id.clone());
    let mut consumer = FauConsumerBuilder::on_environment(kafka_environment)
        .group_id(&group_id)
        .fetch_min_bytes(100_000)
        .fetch_wait_max_ms(25)
        .session_timeout(6000)
        .max_poll_interval(30000)
        .auto_offset_reset(AutoOffsetReset::Latest)
        .auto_commit(true)
        .auto_commit_interval(1000)
        .max_partition_fetch_bytes(1_048_576)
        .build()?;

    consumer.subscribe(&[FauconTopic::FUNDING_RATES_V1])?;

    tracing::info!("🚀 Starting funding rate consumer");

    let mut stream = consumer.stream();
    let mut check_lag_interval = interval(Duration::from_secs(2));

    loop {
        tokio::select! {
            Some(result) = stream.next() => {
                match result {
                    Ok(entry) => {
                        if let Ok(entry) = entry.try_into::<FundingRateEntry>() {
                            let timestamp = chrono::DateTime::from_timestamp_millis(entry.timestamp_ms)
                                .map_or_else(
                                    || {
                                        error!("Invalid timestamp: {}", entry.timestamp_ms);
                                        chrono::NaiveDateTime::default()
                                    },
                                    |dt| dt.naive_utc(),
                                );

                            let funding_rate_entry = NewFundingRate {
                                source: entry.source,
                                pair: entry.pair.to_string(),
                                annualized_rate: entry.annualized_rate,
                                timestamp,
                            };

                            if let Err(e) = funding_rate_tx.send(funding_rate_entry).await {
                                error!("Failed to send funding rate entry: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to consume funding rate entry: {e}");
                    }
                }
            }

            _ = check_lag_interval.tick() => {
                match consumer.lag() {
                    Ok(lag) => {
                        if !lag.is_empty() {
                            let total_lag: i64 = lag.iter().map(|(_, lag)| *lag).sum();
                            metrics_registry.record_consumer_lag(ConsumerType::FundingRate, total_lag);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to get funding rate consumer lag: {e:?}");
                    }
                }
            }
        }
    }
    Ok(())
}

/// Runs a Kafka consumer for open interest entries
#[tracing::instrument(skip_all)]
async fn run_open_interest_consumer(
    group_id: String,
    open_interest_tx: mpsc::Sender<NewOpenInterest>,
    metrics_registry: Arc<IngestorMetricsRegistry>,
) -> anyhow::Result<()> {
    let kafka_environment = FauconEnvironment::Custom(CONFIG.kafka_broker_id.clone());
    let mut consumer = FauConsumerBuilder::on_environment(kafka_environment)
        .group_id(&group_id)
        .fetch_min_bytes(100_000)
        .fetch_wait_max_ms(25)
        .session_timeout(6000)
        .max_poll_interval(30000)
        .auto_offset_reset(AutoOffsetReset::Latest)
        .auto_commit(true)
        .auto_commit_interval(1000)
        .max_partition_fetch_bytes(1_048_576)
        .build()?;

    consumer.subscribe(&[FauconTopic::OPEN_INTEREST_V1])?;

    tracing::info!("🚀 Starting open interest consumer");

    let mut stream = consumer.stream();
    let mut check_lag_interval = interval(Duration::from_secs(2));

    loop {
        tokio::select! {
            Some(result) = stream.next() => {
                match result {
                    Ok(entry) => {
                        if let Ok(entry) = entry.try_into::<OpenInterestEntry>() {
                            let timestamp = chrono::DateTime::from_timestamp_millis(entry.timestamp_ms)
                                .map_or_else(
                                    || {
                                        error!("Invalid timestamp: {}", entry.timestamp_ms);
                                        chrono::NaiveDateTime::default()
                                    },
                                    |dt| dt.naive_utc(),
                                );

                            let open_interest_entry = NewOpenInterest {
                                source: entry.source,
                                pair: entry.pair.to_string(),
                                open_interest_value: entry.open_interest,
                                timestamp,
                            };

                            if let Err(e) = open_interest_tx.send(open_interest_entry).await {
                                error!("Failed to send open interest entry: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to consume open interest entry: {e}");
                    }
                }
            }

            _ = check_lag_interval.tick() => {
                match consumer.lag() {
                    Ok(lag) => {
                        if !lag.is_empty() {
                            let total_lag: i64 = lag.iter().map(|(_, lag)| *lag).sum();
                            metrics_registry.record_consumer_lag(ConsumerType::OpenInterest, total_lag);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to get open interest consumer lag: {e:?}");
                    }
                }
            }
        }
    }
    Ok(())
}
