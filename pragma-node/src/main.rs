use std::sync::Arc;

use dashmap::DashMap;
use dotenvy::dotenv;

use pragma_entities::connection::{ENV_OFFCHAIN_DATABASE_URL, ENV_ONCHAIN_DATABASE_URL};

use pragma_node::caches::CacheRegistry;
use pragma_node::config::config;
use pragma_node::infra::cloud::build_signer;
use pragma_node::infra::rpc::init_rpc_clients;
use pragma_node::metrics::MetricsRegistry;
use pragma_node::state::AppState;

#[tokio::main]
#[tracing::instrument]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let otel_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok();
    pragma_common::telemetry::init_telemetry("pragma-node".into(), otel_endpoint)
        .expect("Failed to initialize telemetry");

    // Init config from env variables
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

    // Build the pragma signer based on cloud environment
    let pragma_signer = build_signer(config.cloud_env(), config.is_production_mode()).await;
    let state = AppState {
        offchain_pool,
        onchain_pool,
        caches: Arc::new(caches),
        pragma_signer,
        metrics: MetricsRegistry::new(),
        publisher_sessions: Arc::new(DashMap::new()),
        rpc_clients: init_rpc_clients(),
    };

    pragma_node::server::run_api_server(config, state).await;

    // Ensure that the tracing provider is shutdown correctly
    opentelemetry::global::shutdown_tracer_provider();

    Ok(())
}
