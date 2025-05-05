use axum::Router;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use utoipa::OpenApi as OpenApiT;
use utoipa_swagger_ui::SwaggerUi;

use crate::handlers::onchain::{
    get_checkpoints::get_onchain_checkpoints, get_entry::get_onchain_entry,
    get_history::get_onchain_history, get_publishers::get_onchain_publishers,
    subscribe_to_ohlc::subscribe_to_onchain_ohlc,
};
use crate::handlers::stream::stream_multi::stream_entry_multi_pair;
use crate::handlers::websocket::{subscribe_to_entry, subscribe_to_price};
use crate::handlers::{
    get_entry, get_funding_rates::get_latest_funding_rate,
    get_historical_funding_rates::get_historical_funding_rates, get_ohlc,
};
use crate::state::AppState;

#[allow(clippy::extra_unused_type_parameters)]
pub fn app_router<T: OpenApiT>(state: AppState) -> Router<AppState> {
    let open_api = T::openapi();
    Router::new()
        .merge(SwaggerUi::new("/node/v1/docs").url("/node/v1/docs/openapi.json", open_api))
        .route("/node", get(root))
        .nest("/node/v1/data", entry_routes(state.clone()))
        .nest("/node/v1/onchain", onchain_routes(state.clone()))
        .nest("/node/v1/aggregation", aggregation_routes(state.clone()))
        .nest("/node/v1/funding_rates", funding_rates_routes(state))
        .fallback(handler_404)
}

async fn root() -> &'static str {
    "Server is running!"
}

async fn handler_404() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        "The requested resource was not found",
    )
}

fn entry_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/{base}/{quote}", get(get_entry))
        .route("/subscribe", get(subscribe_to_entry))
        .route("/price/subscribe", get(subscribe_to_price))
        .route("/multi/stream", get(stream_entry_multi_pair))
        .with_state(state)
}

fn onchain_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/{base}/{quote}", get(get_onchain_entry))
        .route("/history/{base}/{quote}", get(get_onchain_history))
        .route("/checkpoints/{base}/{quote}", get(get_onchain_checkpoints))
        .route("/publishers", get(get_onchain_publishers))
        .route("/ohlc/subscribe", get(subscribe_to_onchain_ohlc))
        .with_state(state)
}

fn aggregation_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/candlestick/{base}/{quote}", get(get_ohlc))
        .with_state(state)
}

fn funding_rates_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/{base}/{quote}", get(get_latest_funding_rate))
        .route("/history/{base}/{quote}", get(get_historical_funding_rates))
        .with_state(state)
}
