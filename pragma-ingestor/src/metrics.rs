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
use tracing::{error, warn};

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

        Arc::new(Self {
            db_operations,
            data_staleness,
            consumer_lag,
        })
    }

    pub(crate) fn record_db_operation(&self, operation: DbOperation, status: Status) {
        self.db_operations.add(
            1,
            &[
                KeyValue::new("operation", operation.to_string()),
                KeyValue::new("status", status.to_string()),
            ],
        );
    }

    pub(crate) fn update_data_staleness(&self, staleness_seconds: f64) {
        self.data_staleness.record(staleness_seconds, &[]);
    }

    pub(crate) fn record_consumer_lag(&self, consumer_type: ConsumerType, lag: i64) {
        self.consumer_lag.record(
            lag,
            &[KeyValue::new("consumer_type", consumer_type.to_string())],
        );
    }
}

#[derive(Display, Clone, Debug)]
pub(crate) enum DbOperation {
    InsertSpotEntries,
    InsertFutureEntries,
    InsertFundingRates,
    InsertOpenInterest,
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

pub(crate) async fn start_data_freshness_monitor(
    pool: Pool,
    metrics_registry: Arc<IngestorMetricsRegistry>,
) {
    let mut interval = interval(Duration::from_secs(30));

    loop {
        interval.tick().await;

        if let Err(e) = check_data_freshness(&pool, &metrics_registry).await {
            error!("Failed to check data freshness: {:?}", e);
        }
    }
}

pub(crate) async fn check_data_freshness(
    pool: &Pool,
    metrics_registry: &IngestorMetricsRegistry,
) -> Result<(), InfraError> {
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

    // Find the most recent timestamp across both tables
    let latest_timestamp = match (latest_spot_timestamp, latest_future_timestamp) {
        (Some(spot), Some(future)) => Some(spot.max(future)),
        (Some(spot), None) => Some(spot),
        (None, Some(future)) => Some(future),
        (None, None) => None,
    };

    if let Some(latest) = latest_timestamp {
        let now = Utc::now();
        let staleness_seconds = (now - latest).num_seconds() as f64;

        metrics_registry.update_data_staleness(staleness_seconds);

        // Log warning if data is more than 5 minutes old
        tracing::info!("Current data staleness: {} seconds", staleness_seconds);
        if staleness_seconds > 300.0 {
            warn!(
                "Data is stale! Latest entry is {} seconds old ({})",
                staleness_seconds,
                latest.format("%Y-%m-%d %H:%M:%S UTC")
            );
        }
    } else {
        warn!("No entries found in database!");
        // Set a high staleness value to trigger alerts
        metrics_registry.update_data_staleness(99999.0);
    }

    Ok(())
}
