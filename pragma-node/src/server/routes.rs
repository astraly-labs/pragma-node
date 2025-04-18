use axum::Router;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use utoipa::OpenApi as OpenApiT;
use utoipa_swagger_ui::SwaggerUi;

use crate::handlers::onchain::{
    get_checkpoints::get_onchain_checkpoints, get_entry::get_onchain_entry,
    get_history::get_onchain_history, get_publishers::get_onchain_publishers,
    subscribe_to_ohlc::subscribe_to_onchain_ohlc,
};
use crate::handlers::stream::stream_multi::stream_entry_multi_pair;
use crate::handlers::websocket::{subscribe_to_entry, subscribe_to_price};
use crate::handlers::{get_entry, get_expiries, get_ohlc};
use crate::state::AppState;

#[allow(clippy::extra_unused_type_parameters)]
pub fn app_router<T: OpenApiT>(state: AppState) -> Router<AppState> {
    let open_api = T::openapi();
    Router::new()
        .merge(SwaggerUi::new("/node/v1/docs").url("/node/v1/docs/openapi.json", open_api))
        .route("/node", get(root))
        .nest("/node/v1/data", data_routes(state.clone()))
        .nest("/node/v1/onchain", onchain_routes(state.clone()))
        .nest("/node/v1/aggregation", aggregation_routes(state))
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

fn data_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/{base}/{quote}", get(get_entry))
        .route("/{base}/{quote}/future_expiries", get(get_expiries))
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
