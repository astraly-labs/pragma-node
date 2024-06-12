use std::time::Duration;

use axum::extract::{Query, State};
use axum::response::IntoResponse;
use pragma_entities::InfraError;
use serde::{Deserialize, Serialize};
use serde_json::json;

use pragma_common::types::{Interval, Network};
use tokio::time::interval;

use crate::infra::repositories::entry_repository::OHLCEntry;
use crate::infra::repositories::onchain_repository::get_ohlc;
use crate::AppState;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};

#[derive(Default, Debug, Serialize, Deserialize)]
enum SubscriptionType {
    #[serde(rename = "subscribe")]
    #[default]
    Subscribe,
    #[serde(rename = "unsubscribe")]
    Unsubscribe,
}

#[derive(Debug, Serialize, Deserialize)]
struct SubscriptionRequest {
    msg_type: SubscriptionType,
    pair: String,
    network: Network,
    interval: Interval,
}

#[derive(Debug, Serialize, Deserialize)]
struct SubscriptionAck {
    msg_type: SubscriptionType,
    pair: String,
    network: Network,
    interval: Interval,
}

/// Interval in milliseconds that the channel will update the client with the latest prices.
const CHANNEL_UPDATE_INTERVAL_IN_MS: u64 = 500;

#[utoipa::path(
    get,
    path = "/node/v1/onchain/ohlc",
    responses(
        (
            status = 200,
            description = "Subscribe to a list of OHLC entries",
            body = [SubscribeToEntryResponse]
        )
    )
)]
pub async fn subscribe_to_onchain_ohlc(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_channel(socket, state))
}

/// Handle the WebSocket channel.
async fn handle_channel(mut socket: WebSocket, state: AppState) {
    let waiting_duration = Duration::from_millis(CHANNEL_UPDATE_INTERVAL_IN_MS);
    let mut update_interval = interval(waiting_duration);
    let mut subscribed_pair: Option<String> = None;
    let mut network = Network::default();
    let mut interval = Interval::default();

    let mut ohlc_to_compute = 10;
    let mut ohlc_data: Vec<OHLCEntry> = Vec::new();

    loop {
        tokio::select! {
            Some(msg) = socket.recv() => {
                if let Ok(Message::Text(text)) = msg {
                    handle_message_received(&mut socket, &state, &mut subscribed_pair, &mut network, &mut interval, text).await;
                }
            },
            _ = update_interval.tick() => {
                match send_ohlc_data(&mut socket, &state, &subscribed_pair, &mut ohlc_data, network, interval, ohlc_to_compute).await {
                    Ok(_) => {
                        // After the first request, we only get the latest interval
                        if !ohlc_data.is_empty() {
                            ohlc_to_compute = 1;
                        }
                    },
                    Err(_) => break
                };
            }
        }
    }
}

/// Handle the message received from the client.
/// Subscribe or unsubscribe to the pairs requested.
async fn handle_message_received(
    socket: &mut WebSocket,
    state: &AppState,
    subscribed_pair: &mut Option<String>,
    network: &mut Network,
    interval: &mut Interval,
    message: String,
) {
    if let Ok(subscription_msg) = serde_json::from_str::<SubscriptionRequest>(&message) {
        match subscription_msg.msg_type {
            SubscriptionType::Subscribe => {
                // TODO: check if the pair exists in the database
                // let existing_pairs =
                //     only_existing_pairs(&state.postgres_pool, subscription_msg.pairs).await;
                *subscribed_pair = Some(subscription_msg.pair.clone());
                *network = subscription_msg.network;
                *interval = subscription_msg.interval;
            }
            SubscriptionType::Unsubscribe => {
                *subscribed_pair = None;
            }
        };
        // We send an ack message to the client with the subscribed pairs (so
        // the client knows which pairs are successfully subscribed).
        if let Ok(ack_message) = serde_json::to_string(&SubscriptionAck {
            msg_type: subscription_msg.msg_type,
            pair: subscription_msg.pair,
            network: subscription_msg.network,
            interval: subscription_msg.interval,
        }) {
            if socket.send(Message::Text(ack_message)).await.is_err() {
                let error_msg = "Message received but could not send ack message.";
                send_error_message(socket, error_msg).await;
            }
        } else {
            let error_msg = "Could not serialize ack message.";
            send_error_message(socket, error_msg).await;
        }
    } else {
        let error_msg = "Invalid message type. Please check the documentation for more info.";
        send_error_message(socket, error_msg).await;
    }
}

/// Send the current median entries to the client.
async fn send_ohlc_data(
    socket: &mut WebSocket,
    state: &AppState,
    subscribed_pair: &Option<String>,
    ohlc_data: &mut Vec<OHLCEntry>,
    network: Network,
    interval: Interval,
    ohlc_to_compute: i64,
) -> Result<(), InfraError> {
    if subscribed_pair.is_none() {
        return Ok(());
    }

    let pair_id = subscribed_pair.as_ref().unwrap();

    let entries = match get_ohlc(
        ohlc_data,
        &state.postgres_pool,
        network,
        pair_id.clone(),
        interval,
        ohlc_to_compute,
    )
    .await
    {
        Ok(()) => ohlc_data,
        Err(e) => {
            send_error_message(socket, &e.to_string()).await;
            return Err(e);
        }
    };
    if let Ok(json_response) = serde_json::to_string(&entries) {
        if socket.send(Message::Text(json_response)).await.is_err() {
            send_error_message(socket, "Could not send prices.").await;
        }
    } else {
        send_error_message(socket, "Could not serialize prices.").await;
    }
    Ok(())
}

/// Send an error message to the client.
/// (Does not close the connection)
async fn send_error_message(socket: &mut WebSocket, error: &str) {
    let error_msg = json!({ "error": error }).to_string();
    socket.send(Message::Text(error_msg)).await.unwrap();
}
