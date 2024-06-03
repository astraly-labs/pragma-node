use std::time::Duration;

use axum::extract::{Query, State};
use axum::response::IntoResponse;
use serde_json::json;

use pragma_common::types::{Interval, Network};

use crate::handlers::entries::utils::currency_pair_to_pair_id;
use crate::handlers::entries::GetOnchainOHLCParams;
use crate::infra::repositories::onchain_repository::get_ohlc;
use crate::utils::PathExtractor;
use crate::AppState;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};

pub const WS_UPDATING_INTERVAL: u64 = 10;

#[utoipa::path(
    get,
    path = "/node/v1/onchain/ws/ohlc/{base}/{quote}",
    responses(
        (
            status = 200,
            description = "Get OHLC data for a pair continuously updated through a ws connection",
            body = GetOnchainOHLCResponse
        )
    ),
    params(
        ("base" = String, Path, description = "Base Asset"),
        ("quote" = String, Path, description = "Quote Asset"),
        ("network" = Network, Query, description = "Network"),
        ("interval" = Interval, Query, description = "Interval of the OHLC data"),
    ),
)]
pub async fn get_onchain_ohlc_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    PathExtractor(pair): PathExtractor<(String, String)>,
    Query(params): Query<GetOnchainOHLCParams>,
) -> impl IntoResponse {
    let pair_id = currency_pair_to_pair_id(&pair.0, &pair.1);
    ws.on_upgrade(move |socket| {
        handle_ohlc_ws(socket, state, pair_id, params.network, params.interval)
    })
}

async fn handle_ohlc_ws(
    mut socket: WebSocket,
    state: AppState,
    pair_id: String,
    network: Network,
    interval: Interval,
) {
    let mut update_interval = tokio::time::interval(Duration::from_secs(WS_UPDATING_INTERVAL));
    loop {
        update_interval.tick().await;
        match get_ohlc(&state.postgres_pool, network, pair_id.clone(), interval).await {
            Ok(response) => {
                if socket
                    .send(Message::Text(serde_json::to_string(&response).unwrap()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Err(e) => {
                if socket
                    .send(Message::Text(json!({ "error": e.to_string() }).to_string()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
    }
}
