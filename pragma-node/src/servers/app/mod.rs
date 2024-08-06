pub(crate) mod routes;

use std::net::SocketAddr;
use tower_http::{
    cors::CorsLayer,
    trace::{DefaultMakeSpan, TraceLayer},
};
use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify, OpenApi,
};

use crate::errors::internal_error;
use crate::{config::Config, handlers, servers::app::routes::app_router, types, AppState};

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

pub async fn run_app_server(config: &Config, state: AppState) {
    #[derive(OpenApi)]
    #[openapi(
        paths(
            handlers::create_entry::create_entries,
            handlers::create_future_entry::create_future_entries,
            handlers::get_entry::get_entry,
            handlers::get_ohlc::get_ohlc,
            handlers::subscribe_to_entry::subscribe_to_entry,
            handlers::get_volatility::get_volatility,
            handlers::onchain::get_entry::get_onchain_entry,
            handlers::onchain::get_history::get_onchain_history,
            handlers::onchain::get_checkpoints::get_onchain_checkpoints,
            handlers::onchain::get_publishers::get_onchain_publishers,
            handlers::onchain::get_ohlc::subscribe_to_onchain_ohlc,
            handlers::merkle_feeds::get_option::get_merkle_feeds_option,
            handlers::merkle_feeds::get_merkle_proof::get_merkle_feeds_proof,
        ),
        components(
            schemas(pragma_entities::dto::Entry, pragma_entities::EntryError),
            schemas(pragma_entities::dto::Publisher, pragma_entities::PublisherError),
            schemas(pragma_entities::error::InfraError),
            schemas(
                handlers::create_entry::CreateEntryRequest,
                handlers::create_entry::CreateEntryResponse,
                handlers::create_future_entry::CreateFutureEntryRequest,
                handlers::create_future_entry::CreateFutureEntryResponse,
                handlers::GetEntryParams,
                handlers::get_entry::GetEntryResponse,
                handlers::subscribe_to_entry::SubscribeToEntryResponse,
                handlers::get_volatility::GetVolatilityResponse,
                handlers::get_ohlc::GetOHLCResponse,
                handlers::onchain::get_entry::GetOnchainEntryParams,
                handlers::onchain::get_entry::GetOnchainEntryResponse,
                handlers::onchain::get_checkpoints::GetOnchainCheckpointsParams,
                handlers::onchain::get_checkpoints::GetOnchainCheckpointsResponse,
                handlers::onchain::get_publishers::GetOnchainPublishersParams,
                handlers::onchain::get_publishers::GetOnchainPublishersResponse,
                handlers::onchain::get_ohlc::GetOnchainOHLCResponse,
                handlers::onchain::get_history::GetOnchainHistoryParams,
                handlers::onchain::get_history::GetOnchainHistoryResponse,
                handlers::merkle_feeds::get_option::GetOptionQuery,
                handlers::merkle_feeds::get_option::GetOptionResponse,
                handlers::merkle_feeds::get_merkle_proof::GetMerkleProofQuery,
                handlers::merkle_feeds::get_merkle_proof::GetMerkleProofResponse,
            ),
            schemas(
                types::entries::BaseEntry,
                types::entries::Entry,
                types::entries::PerpEntry,
                types::entries::FutureEntry,
                handlers::onchain::get_entry::OnchainEntry,
                handlers::onchain::get_checkpoints::Checkpoint,
                handlers::onchain::get_publishers::Publisher,
                handlers::onchain::get_publishers::PublisherEntry,
                handlers::onchain::get_history::GetOnchainHistoryEntry,
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

    tracing::info!("🚀 API started at http://{}", socket_addr);
    tokio::spawn(async move {
        axum::Server::bind(&socket_addr)
            .serve(app.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .map_err(internal_error)
            .unwrap()
    });
}
