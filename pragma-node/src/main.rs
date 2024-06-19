use deadpool_diesel::postgres::Pool;
use pragma_entities::connection::{ENV_OFFCHAIN_DATABASE_URL, ENV_ONCHAIN_DATABASE_URL};
use starknet::signers::SigningKey;
use utils::PragmaSignerBuilder;

use crate::config::config;

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

    let state = AppState {
        offchain_pool,
        onchain_pool,
        pragma_signer,
    };

    tokio::join!(
        servers::app::run_app_server(config, state.clone()),
        servers::metrics::run_metrics_server(config)
    );
}
