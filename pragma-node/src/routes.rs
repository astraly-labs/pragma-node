use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;

use utoipa::OpenApi as OpenApiT;
use utoipa_swagger_ui::SwaggerUi;

use crate::handlers::entries::{create_entries, get_entry, get_ohlc, get_volatility};
use crate::AppState;

pub fn app_router<T: OpenApiT>(state: AppState) -> Router<AppState> {
    let open_api = T::openapi();
    Router::new()
        .merge(SwaggerUi::new("/node/swagger-ui").url("/node/api-docs/openapi.json", open_api))
        .route("/node", get(root))
        .nest("/node/v1/data", data_routes(state.clone()))
        .nest("/node/v1/aggregation", aggregation_routes(state.clone()))
        .nest("/node/v1/volatility", volatility_routes(state.clone()))
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
        .route("/publish", post(create_entries))
        .route("/:base/:quote", get(get_entry))
        .with_state(state)
}

fn volatility_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/:base/:quote", get(get_volatility))
        .with_state(state)
}

fn aggregation_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/candlestick/:base/:quote", get(get_ohlc))
        .with_state(state)
}
