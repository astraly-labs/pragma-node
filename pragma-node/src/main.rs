pub mod caches;
pub mod config;
pub mod constants;
pub mod errors;
pub mod handlers;
pub mod infra;
pub mod metrics;
pub mod server;
pub mod utils;

use dashmap::DashMap;
use dotenvy::dotenv;
use handlers::publish_entry_ws::PublisherSession;
use metrics::MetricsRegistry;
use std::fmt;
use std::sync::Arc;

use caches::CacheRegistry;
use deadpool_diesel::postgres::Pool;
use starknet::signers::SigningKey;

use pragma_entities::connection::{ENV_OFFCHAIN_DATABASE_URL, ENV_ONCHAIN_DATABASE_URL};

use crate::config::config;
use crate::utils::PragmaSignerBuilder;

#[derive(Clone)]
pub struct AppState {
    // Databases pools
    offchain_pool: Pool,
    onchain_pool: Pool,
    // Database caches
    caches: Arc<CacheRegistry>,
    // Pragma Signer used for StarkEx signing
    pragma_signer: Option<SigningKey>,
    // Metrics
    metrics: Arc<MetricsRegistry>,
    // Publisher sessions
    publisher_sessions: Arc<DashMap<String, PublisherSession>>,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState")
            .field("caches", &self.caches)
            .field("pragma_signer", &self.pragma_signer)
            .field("metrics", &self.metrics)
            .finish_non_exhaustive()
    }
}

#[tokio::main]
#[tracing::instrument]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    // We export our telemetry - so we can monitor the API through Signoz.
    let otel_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://signoz.dev.pragma.build:4317".to_string());
    pragma_common::telemetry::init_telemetry("pragma-node".into(), otel_endpoint, None)?;

    let config = config().await;

    // Init the database pools
    let offchain_pool =
        pragma_entities::connection::init_pool("pragma-node-api", ENV_OFFCHAIN_DATABASE_URL)
            .expect("can't init offchain database pool");
    pragma_entities::db::run_migrations(&offchain_pool).await;
    let onchain_pool =
        pragma_entities::connection::init_pool("pragma-node-api", ENV_ONCHAIN_DATABASE_URL)
            .expect("can't init onchain database pool");

    // Init the database caches
    let caches = CacheRegistry::new();

    // Build the pragma signer
    let signer_builder = if config.is_production_mode() {
        PragmaSignerBuilder::new().production_mode()
    } else {
        PragmaSignerBuilder::new().non_production_mode()
    };
    let pragma_signer = signer_builder.build().await;

    let state = AppState {
        offchain_pool,
        onchain_pool,
        caches: Arc::new(caches),
        pragma_signer,
        metrics: MetricsRegistry::new(),
        publisher_sessions: Arc::new(DashMap::new()),
    };

    server::run_api_server(config, state).await;

    // Ensure that the tracing provider is shutdown correctly
    opentelemetry::global::shutdown_tracer_provider();

    Ok(())
}
