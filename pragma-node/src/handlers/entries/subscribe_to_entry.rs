use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tokio::time::interval;

use crate::handlers::entries::SubscribeToEntryResponse;
use crate::AppState;

const UPDATE_INTERVAL_IN_MS: u64 = 500;

#[derive(Default, Debug, Serialize, Deserialize)]
enum SubscriptionType {
    #[serde(rename = "subscribe")]
    #[default]
    Subscribe,
    #[serde(rename = "unsubscribe")]
    Unsubscribe,
}

#[derive(Debug, Serialize, Deserialize)]
struct SubscriptionMessage {
    msg_type: SubscriptionType,
    pairs: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SubscriptionAck {
    msg_type: SubscriptionType,
    pairs: Vec<String>,
}

#[utoipa::path(
    get,
    path = "/node/v1/data/subscribe",
    responses(
        (
            status = 200,
            description = "Subscribe to a list of entries",
            body = [SubscribeToEntryResponse]
        )
    )
)]
pub async fn subscribe_to_entry(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_subscription(socket, state))
}

async fn handle_subscription(mut socket: WebSocket, _state: AppState) {
    let waiting_duration = Duration::from_millis(UPDATE_INTERVAL_IN_MS);
    let mut update_interval = interval(waiting_duration);

    let mut subscriptions: Vec<String> = Vec::new();

    // TODO(akhercha): refinements for readability
    loop {
        tokio::select! {
            Some(msg) = socket.recv() => {
                if let Ok(Message::Text(text)) = msg {
                    // Handle subscription/unsubscription messages
                    if let Ok(subscription_msg) = serde_json::from_str::<SubscriptionMessage>(&text) {
                        match subscription_msg.msg_type {
                            SubscriptionType::Subscribe => {
                                subscriptions.extend(subscription_msg.pairs.clone());
                            },
                            SubscriptionType::Unsubscribe => {
                                subscriptions.retain(|pair| !subscription_msg.pairs.contains(pair));
                            },
                        };
                        // Acknowledge subscription/unsubscription
                        let ack_message = serde_json::to_string(&SubscriptionAck {
                            msg_type: subscription_msg.msg_type,
                            pairs: subscriptions.clone(),
                        }).unwrap();
                        if socket.send(Message::Text(ack_message)).await.is_err() {
                            // Exit if there is an error sending message
                            break;
                        }
                    }
                // Break channel if there's an error receiving messages
                } else if msg.is_err() {
                    break;
                }
            },
            // Update entries logic every X milliseconds
            _ = update_interval.tick() => {
                // TODO(akhercha): Implement the logic to fetch entries for the given subscriptions
                let entries = SubscribeToEntryResponse::default();
                let json_response = serde_json::to_string(&entries).unwrap();
                if socket.send(Message::Text(json_response)).await.is_err() {
                    break;
                }
            }
        }
    }
}
