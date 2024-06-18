use deadpool_diesel::postgres::Pool;
use pragma_entities::connection::{ENV_OFFCHAIN_DATABASE_URL, ENV_ONCHAIN_DATABASE_URL};
use starknet::signers::SigningKey;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
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
mod types;
mod utils;

#[derive(Clone)]
pub struct AppState {
    timescale_pool: Pool,
    postgres_pool: Pool,
    pragma_signer: Option<SigningKey>,
}

#[tokio::main]
async fn main() {
    pragma_common::tracing::init_tracing();

    #[derive(OpenApi)]
    #[openapi(
        paths(
            handlers::entries::create_entry::create_entries,
            handlers::entries::create_future_entry::create_future_entries,
            handlers::entries::get_entry::get_entry,
            handlers::entries::get_ohlc::get_ohlc,
            handlers::entries::subscribe_to_entry::subscribe_to_entry,
            handlers::entries::get_volatility::get_volatility,
            handlers::entries::get_onchain::get_onchain,
            handlers::entries::get_onchain::checkpoints::get_onchain_checkpoints,
            handlers::entries::get_onchain::publishers::get_onchain_publishers,
            handlers::entries::get_onchain::ohlc::subscribe_to_onchain_ohlc,
            handlers::entries::test_ws::test_ws,
        ),
        components(
            schemas(pragma_entities::dto::Entry, pragma_entities::EntryError),
            schemas(pragma_entities::dto::Publisher, pragma_entities::PublisherError),
            schemas(pragma_entities::error::InfraError),
            schemas(
                handlers::entries::CreateEntryRequest,
                handlers::entries::CreateEntryResponse,
                handlers::entries::CreateFutureEntryRequest,
                handlers::entries::CreateFutureEntryResponse,
                handlers::entries::GetEntryParams,
                handlers::entries::GetEntryResponse,
                handlers::entries::SubscribeToEntryResponse,
                handlers::entries::GetVolatilityResponse,
                handlers::entries::GetOHLCResponse,
                handlers::entries::GetOnchainParams,
                handlers::entries::GetOnchainResponse,
                handlers::entries::GetOnchainCheckpointsParams,
                handlers::entries::GetOnchainCheckpointsResponse,
                handlers::entries::GetOnchainPublishersParams,
                handlers::entries::GetOnchainPublishersResponse,
                handlers::entries::GetOnchainOHLCResponse,
            ),
            schemas(
                types::entries::BaseEntry,
                types::entries::Entry,
                types::entries::PerpEntry,
                types::entries::FutureEntry,
                handlers::entries::OnchainEntry,
                handlers::entries::Checkpoint,
                handlers::entries::Publisher,
                handlers::entries::PublisherEntry,
            ),
            schemas(
                pragma_common::types::AggregationMode,
                pragma_common::types::Interval,
                pragma_common::types::Network,
                pragma_common::types::DataType,
            ),
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
        pragma_entities::connection::init_pool("pragma-node-api", ENV_OFFCHAIN_DATABASE_URL)
            .expect("can't init offchain database pool");
    pragma_entities::db::run_migrations(&timescale_pool).await;

    let postgres_pool =
        pragma_entities::connection::init_pool("pragma-node-api", ENV_ONCHAIN_DATABASE_URL)
            .expect("can't init onchain database pool");

    // TODO(#54): Build the signer using a builder pattern
    let pragma_signer: Option<SigningKey> = if config.is_production_mode() {
        utils::build_pragma_signer_from_aws().await
    } else {
        Some(SigningKey::from_random())
    };

    let state = AppState {
        timescale_pool,
        postgres_pool,
        pragma_signer,
    };

    let app = app_router::<ApiDoc>(state.clone())
        .with_state(state)
        // Logging so we can see whats going on
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        // Permissive CORS layer to allow all origins
        .layer(CorsLayer::permissive());

    let host = config.server_host();
    let port = config.server_port();

    let address = format!("{}:{}", host, port);

    let socket_addr: SocketAddr = address.parse().unwrap();

    tracing::info!("listening on http://{}", socket_addr);
    axum::Server::bind(&socket_addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .map_err(internal_error)
        .unwrap()
}
