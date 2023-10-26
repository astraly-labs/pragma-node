use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;

use utoipa::OpenApi as OpenApiT;
use utoipa_swagger_ui::SwaggerUi;

use crate::handlers::entries::{convert_amount, create_entries, get_entry, get_volatility};
use crate::AppState;

pub fn app_router<T: OpenApiT>(state: AppState) -> Router<AppState> {
    let open_api = T::openapi();
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", open_api))
        .route("/", get(root))
        .nest("/v1/data", data_routes(state.clone()))
        .nest("/v1/volatility", volatility_routes(state.clone()))
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
        .route("/:quote/:base", get(get_entry))
        .route("/:quote/:base/:amount", get(convert_amount))
        .with_state(state)
}

fn volatility_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/:quote/:base", get(get_volatility))
        .with_state(state)
}
