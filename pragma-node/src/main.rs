mod config;
mod constants;
mod errors;
mod handlers;
mod infra;
mod metrics;
mod servers;
mod types;
mod utils;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use deadpool_diesel::postgres::Pool;
use moka::future::Cache;
use starknet::signers::SigningKey;

use pragma_entities::connection::{ENV_OFFCHAIN_DATABASE_URL, ENV_ONCHAIN_DATABASE_URL};

use crate::config::config;
use crate::constants::{
    PUBLISHERS_UDPATES_CACHE_TIME_TO_IDLE_IN_SECONDS,
    PUBLISHERS_UDPATES_CACHE_TIME_TO_LIVE_IN_SECONDS,
};
use crate::infra::repositories::onchain_repository::RawPublisherUpdates;
use crate::metrics::MetricsRegistry;
use crate::utils::PragmaSignerBuilder;
use types::ws::metrics::WsMetrics;

#[derive(Clone)]
pub struct AppState {
    // Databases pools
    offchain_pool: Pool,
    onchain_pool: Pool,
    // Redis connection
    #[allow(dead_code)]
    redis_client: Option<Arc<redis::Client>>,
    // Database caches
    publishers_updates_cache: Cache<String, HashMap<String, RawPublisherUpdates>>,
    // Pragma Signer used for StarkEx signing
    pragma_signer: Option<SigningKey>,
    // Metrics
    ws_metrics: Arc<WsMetrics>,
}

#[tokio::main]
async fn main() {
    pragma_common::tracing::init_tracing();

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
    let publishers_updates_cache = Cache::builder()
        .time_to_live(Duration::from_secs(
            PUBLISHERS_UDPATES_CACHE_TIME_TO_LIVE_IN_SECONDS,
        )) // 30 minutes
        .time_to_idle(Duration::from_secs(
            PUBLISHERS_UDPATES_CACHE_TIME_TO_IDLE_IN_SECONDS,
        )) // 5 minutes
        .build();

    // Build the pragma signer
    let signer_builder = if config.is_production_mode() {
        PragmaSignerBuilder::new().production_mode()
    } else {
        PragmaSignerBuilder::new().non_production_mode()
    };
    let pragma_signer = signer_builder.build().await;

    // Init the redis client - Optionnal, only for endpoints that interact with Redis.
    // TODO(akhercha): See with Hithem for production mode
    let redis_client = match pragma_entities::connection::init_redis_client(
        config.redis_host(),
        config.redis_port(),
    ) {
        Ok(client) => Some(Arc::new(client)),
        Err(_) => None,
    };

    // Create the Metrics registry
    let metrics_registry = MetricsRegistry::new();
    let ws_metrics = WsMetrics::new(&metrics_registry).expect("Failed to create WsMetrics");

    let state = AppState {
        offchain_pool,
        onchain_pool,
        redis_client,
        publishers_updates_cache,
        pragma_signer,
        ws_metrics: Arc::new(ws_metrics),
    };

    tokio::join!(
        servers::app::run_app_server(config, state),
        servers::metrics::run_metrics_server(config, metrics_registry)
    );
}
