use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;

use crate::handlers::entries::{create_entry, get_entry};
use crate::AppState;

pub fn app_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(root))
        .nest("/v1/data", data_routes(state.clone()))
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
        .route("/publish", post(create_entry))
        .route("/:quote/:base", get(get_entry))
        .with_state(state)
}
