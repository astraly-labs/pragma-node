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
use crate::{config::Config, handlers, routes::app_router, types, AppState};

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
    println!("{}", ApiDoc::openapi().to_pretty_json().unwrap());

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
