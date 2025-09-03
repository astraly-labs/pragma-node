use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use deadpool_diesel::postgres::Pool;
use opentelemetry::{
    KeyValue,
    metrics::{Counter, Gauge},
};
use pragma_entities::InfraError;
use strum::Display;
use tokio::time::interval;
use tracing::{debug, error, warn};

#[derive(Debug)]
pub(crate) struct IngestorMetricsRegistry {
    pub db_operations: Counter<u64>,
    pub data_staleness: Gauge<f64>,
    pub consumer_lag: Gauge<i64>,
}

impl IngestorMetricsRegistry {
    pub(crate) fn new() -> Arc<Self> {
        let meter = opentelemetry::global::meter("pragma-ingestor-meter");

        let db_operations = meter
            .u64_counter("ingestor_db_operations_total")
            .with_description("Number of database operations performed by the ingestor")
            .with_unit("count")
            .init();

        let data_staleness = meter
            .f64_gauge("ingestor_data_staleness_seconds")
            .with_description("Age of the latest entry in the database")
            .with_unit("s")
            .init();

        let consumer_lag = meter
            .i64_gauge("ingestor_consumer_lag_total")
            .with_description("Total lag of Kafka consumers")
            .with_unit("count")
            .init();

        debug!("Created OpenTelemetry metrics: db_operations, data_staleness, consumer_lag");

        Arc::new(Self {
            db_operations,
            data_staleness,
            consumer_lag,
        })
    }

    pub(crate) fn record_db_operation(&self, operation: InsertDbOperation, status: Status) {
        debug!(
            "Recording DB operation: {:?} with status: {:?}",
            operation, status
        );
        self.db_operations.add(
            1,
            &[
                KeyValue::new("operation", operation.to_string()),
                KeyValue::new("status", status.to_string()),
            ],
        );
    }

    pub(crate) fn update_data_staleness(&self, staleness_seconds: f64, data_type: DataType) {
        debug!(
            "Updating data staleness for {:?}: {} seconds",
            data_type, staleness_seconds
        );
        self.data_staleness.record(
            staleness_seconds,
            &[KeyValue::new("data_type", data_type.to_string())],
        );
    }

    pub(crate) fn record_consumer_lag(&self, consumer_type: ConsumerType, lag: i64) {
        debug!("Recording consumer lag: {:?} = {}", consumer_type, lag);
        self.consumer_lag.record(
            lag,
            &[KeyValue::new("consumer_type", consumer_type.to_string())],
        );
    }
}

#[derive(Display, Clone, Debug)]
pub(crate) enum InsertDbOperation {
    SpotEntries,
    FutureEntries,
    FundingRates,
    OpenInterest,
}

#[derive(Display, Clone, Debug)]
pub(crate) enum ConsumerType {
    Price,
    FundingRate,
    OpenInterest,
}

#[derive(Display, Clone, Debug)]
pub(crate) enum Status {
    Success,
    Error,
}

#[derive(Display, Clone, Debug)]
pub(crate) enum DataType {
    Entries,
    FutureEntries,
}

pub(crate) async fn start_data_freshness_monitor(
    pool: Pool,
    metrics_registry: Arc<IngestorMetricsRegistry>,
) {
    debug!("Starting data freshness monitor");
    let mut interval = interval(Duration::from_secs(30));

    loop {
        interval.tick().await;

        if let Err(e) = check_data_freshness(&pool, &metrics_registry).await {
            error!("Failed to check data freshness: {:?}", e);
        }
    }
}

#[allow(clippy::cognitive_complexity)]
pub(crate) async fn check_data_freshness(
    pool: &Pool,
    metrics_registry: &IngestorMetricsRegistry,
) -> Result<(), InfraError> {
    debug!("Checking data freshness...");
    let conn = pool.get().await?;

    // Get latest timestamps from both tables using efficient ORDER BY + LIMIT
    let latest_spot_timestamp = conn
        .interact(
            |conn| -> Result<Option<DateTime<Utc>>, diesel::result::Error> {
                use diesel::prelude::*;
                use pragma_entities::schema::entries;

                entries::table
                    .select(entries::timestamp)
                    .order(entries::timestamp.desc())
                    .limit(1)
                    .first::<DateTime<Utc>>(conn)
                    .optional()
            },
        )
        .await??;

    let latest_future_timestamp = conn
        .interact(
            |conn| -> Result<Option<DateTime<Utc>>, diesel::result::Error> {
                use diesel::prelude::*;
                use pragma_entities::schema::future_entries;

                future_entries::table
                    .select(future_entries::timestamp)
                    .order(future_entries::timestamp.desc())
                    .limit(1)
                    .first::<DateTime<Utc>>(conn)
                    .optional()
            },
        )
        .await??;

    let now = Utc::now();

    // Update staleness for spot entries
    if let Some(latest_spot) = latest_spot_timestamp {
        let staleness_seconds = (now - latest_spot).num_seconds() as f64;
        metrics_registry.update_data_staleness(staleness_seconds, DataType::Entries);

        tracing::info!("Spot entries data staleness: {} seconds", staleness_seconds);
        if staleness_seconds > 300.0 {
            warn!(
                "Spot entries data is stale! Latest entry is {} seconds old ({})",
                staleness_seconds,
                latest_spot.format("%Y-%m-%d %H:%M:%S UTC")
            );
        }
    } else {
        warn!("No spot entries found in database!");
        metrics_registry.update_data_staleness(99999.0, DataType::Entries);
    }

    // Update staleness for future entries
    if let Some(latest_future) = latest_future_timestamp {
        let staleness_seconds = (now - latest_future).num_seconds() as f64;
        metrics_registry.update_data_staleness(staleness_seconds, DataType::FutureEntries);

        tracing::info!(
            "Future entries data staleness: {} seconds",
            staleness_seconds
        );
        if staleness_seconds > 300.0 {
            warn!(
                "Future entries data is stale! Latest entry is {} seconds old ({})",
                staleness_seconds,
                latest_future.format("%Y-%m-%d %H:%M:%S UTC")
            );
        }
    } else {
        warn!("No future entries found in database!");
        metrics_registry.update_data_staleness(99999.0, DataType::FutureEntries);
    }

    Ok(())
}
