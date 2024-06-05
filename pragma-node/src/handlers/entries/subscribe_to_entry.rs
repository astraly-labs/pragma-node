use std::default;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use serde_json::json;

use pragma_entities::EntryError;
use tokio::time::{interval, Interval};

use crate::handlers::entries::{SubscribeToEntryParams, SubscribeToEntryResponse};
use crate::AppState;

const ORACLE_PRICES_TICK_TYPE: &str = "ORACLE_PRICES_TICK";

const UPDATE_INTERVAL_IN_MS: u64 = 500;

#[utoipa::path(
    get,
    path = "/node/v1/data/subscribe",
    responses(
        (
            status = 200,
            description = "Subscribe to a list of entries",
            body = [SubscribeToEntryResponse]
        )
    ),
    params(
        SubscribeToEntryParams,
    ),
)]
pub async fn subscribe_to_entry(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(params): Query<SubscribeToEntryParams>,
) -> impl IntoResponse {
    tracing::info!("New subscription for entries");
    let pairs = params.pairs;
    ws.on_upgrade(move |socket| handle_subscription(socket, state, pairs))
}

async fn handle_subscription(mut socket: WebSocket, _state: AppState, _pairs: Vec<String>) {
    let interval_duration = Duration::from_millis(UPDATE_INTERVAL_IN_MS);
    let mut update_interval = interval(interval_duration);
    let entries = SubscribeToEntryResponse::default();

    // TODO: search how to trigger an update when new data is published
    loop {
        let json_response = serde_json::to_string(&entries).unwrap();

        // TODO: update the entries for every pairs

        let response_status = socket.send(Message::Text(json_response)).await;
        if response_status.is_err() {
            break;
        }
        update_interval.tick().await;
    }
}
