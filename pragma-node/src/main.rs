use deadpool_diesel::postgres::Pool;
use pragma_entities::connection::{ENV_POSTGRES_DATABASE_URL, ENV_TS_DATABASE_URL};
use std::net::SocketAddr;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::Modify;
use utoipa::OpenApi;

use crate::config::config;
use crate::errors::internal_error;
use crate::routes::app_router;

mod config;
mod errors;
mod handlers;
mod infra;
mod routes;
mod utils;

#[derive(Clone)]
pub struct AppState {
    timescale_pool: Pool,
    postgres_pool: Pool,
}

#[tokio::main]
async fn main() {
    pragma_common::tracing::init_tracing();

    #[derive(OpenApi)]
    #[openapi(
        paths(
            handlers::entries::create_entry::create_entries,
            handlers::entries::get_entry::get_entry,
            handlers::entries::get_ohlc::get_ohlc,
            handlers::entries::get_volatility::get_volatility,
            handlers::entries::get_onchain::get_onchain,
            handlers::entries::get_onchain::checkpoints::get_onchain_checkpoints,
        ),
        components(
            schemas(pragma_entities::dto::Entry, pragma_entities::EntryError),
            schemas(pragma_entities::dto::Publisher, pragma_entities::PublisherError),
            schemas(pragma_entities::error::InfraError),
            schemas(
                handlers::entries::CreateEntryRequest,
                handlers::entries::CreateEntryResponse,
                handlers::entries::GetEntryParams,
                handlers::entries::GetEntryResponse,
                handlers::entries::GetVolatilityResponse,
                handlers::entries::GetOHLCResponse,
                handlers::entries::GetOnchainParams,
                handlers::entries::GetOnchainResponse,
                handlers::entries::GetOnchainCheckpointsParams,
                handlers::entries::GetOnchainCheckpointsResponse,
            ),
            schemas(
                handlers::entries::Entry,
                handlers::entries::BaseEntry,
                handlers::entries::OnchainEntry,
            ),
            schemas(handlers::entries::Checkpoint),
            schemas(handlers::entries::Network),
            schemas(handlers::entries::AggregationMode),
            schemas(handlers::entries::Interval),
        ),
        modifiers(&SecurityAddon),
        tags(
            (name = "pragma-node", description = "Pragma Node API")
        )
    )]
    struct ApiDoc;

    struct SecurityAddon;

    impl Modify for SecurityAddon {
        fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
            if let Some(components) = openapi.components.as_mut() {
                components.add_security_scheme(
                    "api_key",
                    SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("pragma_apikey"))),
                )
            }
        }
    }

    println!("{}", ApiDoc::openapi().to_pretty_json().unwrap());
    let config = config().await;

    let timescale_pool =
        pragma_entities::connection::init_pool("pragma-node-api", ENV_TS_DATABASE_URL)
            .expect("can't init timescale (offchain db) pool");
    pragma_entities::db::run_migrations(&timescale_pool).await;

    let postgres_pool =
        pragma_entities::connection::init_pool("pragma-node-api", ENV_POSTGRES_DATABASE_URL)
            .expect("can't init postgres (onchain db) pool");

    let state = AppState {
        timescale_pool,
        postgres_pool,
    };

    let app = app_router::<ApiDoc>(state.clone()).with_state(state);

    let host = config.server_host();
    let port = config.server_port();

    let address = format!("{}:{}", host, port);

    let socket_addr: SocketAddr = address.parse().unwrap();

    tracing::info!("listening on http://{}", socket_addr);
    axum::Server::bind(&socket_addr)
        .serve(app.into_make_service())
        .await
        .map_err(internal_error)
        .unwrap()
}
