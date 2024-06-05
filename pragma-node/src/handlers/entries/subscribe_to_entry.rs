use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use tokio::time::interval;

use pragma_entities::Entry;

use crate::handlers::entries::{SubscribeToEntryParams, SubscribeToEntryResponse};
use crate::AppState;

use super::SignedOraclePrice;

const _ORACLE_PRICES_TICK_TYPE: &str = "ORACLE_PRICES_TICK";
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
    if params.pairs.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "No pairs specified for subscription"})),
        ));
    }
    Ok(ws.on_upgrade(move |socket| handle_subscription(socket, state, params.pairs)))
}

async fn handle_subscription(mut socket: WebSocket, _state: AppState, _pairs: Vec<String>) {
    let waiting_duration = Duration::from_millis(UPDATE_INTERVAL_IN_MS);
    let mut update_interval = interval(waiting_duration);
    let entries = SubscribeToEntryResponse::default();
    // TODO(akhercha): trigger an update when new data is published
    loop {
        // TODO(akhercha): update the entries for every pairs
        let _: Vec<Entry> = vec![];
        // TODO(akhercha): convert Vec<Entry> to Vec<SignedOraclePrice>
        let _: Vec<SignedOraclePrice> = vec![];
        // TODO(akhercha): update the response with the new entries
        let json_response = serde_json::to_string(&entries).unwrap();
        let response_status = socket.send(Message::Text(json_response)).await;
        if response_status.is_err() {
            break;
        }
        update_interval.tick().await;
    }
}
