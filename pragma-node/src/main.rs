use deadpool_diesel::postgres::Pool;
use moka::future::Cache;
use starknet::signers::SigningKey;
use std::collections::HashMap;
use std::time::Duration;

use pragma_entities::connection::{ENV_OFFCHAIN_DATABASE_URL, ENV_ONCHAIN_DATABASE_URL};

use utils::PragmaSignerBuilder;

use crate::config::config;
use crate::handlers::entries::constants::{
    PUBLISHERS_UDPATES_CACHE_TIME_TO_IDLE_IN_SECONDS,
    PUBLISHERS_UDPATES_CACHE_TIME_TO_LIVE_IN_SECONDS,
};

use crate::infra::repositories::onchain_repository::RawPublisherUpdates;

mod config;
mod errors;
mod handlers;
mod infra;
mod routes;
mod servers;
mod types;
mod utils;

#[derive(Clone)]
pub struct AppState {
    offchain_pool: Pool,
    onchain_pool: Pool,
    pragma_signer: Option<SigningKey>,
    publishers_updates_cache: Cache<String, HashMap<String, RawPublisherUpdates>>,
}

#[tokio::main]
async fn main() {
    pragma_common::tracing::init_tracing();

    let config = config().await;

    let offchain_pool =
        pragma_entities::connection::init_pool("pragma-node-api", ENV_OFFCHAIN_DATABASE_URL)
            .expect("can't init offchain database pool");
    pragma_entities::db::run_migrations(&offchain_pool).await;

    let onchain_pool =
        pragma_entities::connection::init_pool("pragma-node-api", ENV_ONCHAIN_DATABASE_URL)
            .expect("can't init onchain database pool");

    let signer_builder = if config.is_production_mode() {
        PragmaSignerBuilder::new().production_mode()
    } else {
        PragmaSignerBuilder::new().non_production_mode()
    };
    let pragma_signer = signer_builder.build().await;

    let publishers_updates_cache = Cache::builder()
        .time_to_live(Duration::from_secs(
            PUBLISHERS_UDPATES_CACHE_TIME_TO_LIVE_IN_SECONDS,
        )) // 30 minutes
        .time_to_idle(Duration::from_secs(
            PUBLISHERS_UDPATES_CACHE_TIME_TO_IDLE_IN_SECONDS,
        )) // 5 minutes
        .build();

    let state = AppState {
        offchain_pool,
        onchain_pool,
        pragma_signer,
        publishers_updates_cache,
    };

    tokio::join!(
        servers::app::run_app_server(config, state.clone()),
        servers::metrics::run_metrics_server(config)
    );
}
