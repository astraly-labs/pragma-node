use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use pragma_entities::EntryError;
use serde::{Deserialize, Serialize};
use serde_json::json;
use starknet::signers::SigningKey;
use tokio::time::interval;

use crate::handlers::entries::SubscribeToEntryResponse;
use crate::infra::repositories::entry_repository::get_current_median_entries_with_components;
use crate::utils::get_entry_hash;
use crate::AppState;

use super::AssetOraclePrice;

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
struct SubscriptionRequest {
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
    ws.on_upgrade(move |socket| handle_channel(socket, state))
}

async fn handle_channel(mut socket: WebSocket, state: AppState) {
    let waiting_duration = Duration::from_millis(UPDATE_INTERVAL_IN_MS);
    let mut update_interval = interval(waiting_duration);
    let mut subscribed_pairs: Vec<String> = Vec::new();

    loop {
        tokio::select! {
            Some(msg) = socket.recv() => {
                if let Ok(Message::Text(text)) = msg {
                    handle_messages_received(&mut socket, &mut subscribed_pairs, text).await;
                }
            },
            _ = update_interval.tick() => {
                match handle_entries_refresh(&mut socket, &state, &subscribed_pairs).await {
                    Ok(_) => {},
                    Err(_) => break
                };
            }
        }
    }
}

async fn handle_messages_received(
    socket: &mut WebSocket,
    subscribed_pairs: &mut Vec<String>,
    message: String,
) {
    if let Ok(subscription_msg) = serde_json::from_str::<SubscriptionRequest>(&message) {
        // TODO(akhercha): send errors for pairs not handled by Pragma
        match subscription_msg.msg_type {
            SubscriptionType::Subscribe => {
                subscribed_pairs.extend(subscription_msg.pairs.clone());
                subscribed_pairs.dedup();
            }
            SubscriptionType::Unsubscribe => {
                subscribed_pairs.retain(|pair| !subscription_msg.pairs.contains(pair));
            }
        };
        let ack_message = serde_json::to_string(&SubscriptionAck {
            msg_type: subscription_msg.msg_type,
            pairs: subscribed_pairs.clone(),
        })
        .unwrap();
        if socket.send(Message::Text(ack_message)).await.is_err() {
            let error_msg = "Message received but could not send ack message.";
            socket
                .send(Message::Text(json!({ "error": error_msg }).to_string()))
                .await
                .unwrap();
        }
    } else {
        let error_msg = "Invalid message type. Please check the documentation for more info.";
        socket
            .send(Message::Text(json!({ "error": error_msg }).to_string()))
            .await
            .unwrap();
    }
}

async fn handle_entries_refresh(
    socket: &mut WebSocket,
    state: &AppState,
    subscribed_pairs: &[String],
) -> Result<(), EntryError> {
    if subscribed_pairs.is_empty() {
        return Ok(());
    }
    let entries = match get_subscribed_pairs_entries(state, subscribed_pairs).await {
        Ok(response) => response,
        Err(e) => {
            socket
                .send(Message::Text(json!({ "error": e.to_string() }).to_string()))
                .await
                .unwrap();
            return Err(e);
        }
    };
    let json_response = serde_json::to_string(&entries).unwrap();
    if socket.send(Message::Text(json_response)).await.is_err() {
        let error_msg = "Could not send prices.";
        socket
            .send(Message::Text(json!({ "error": error_msg }).to_string()))
            .await
            .unwrap();
    }
    Ok(())
}

async fn get_subscribed_pairs_entries(
    state: &AppState,
    subscribed_pairs: &[String],
) -> Result<SubscribeToEntryResponse, EntryError> {
    let median_entries =
        get_current_median_entries_with_components(&state.timescale_pool, subscribed_pairs)
            .await
            .map_err(|e| e.to_entry_error(&subscribed_pairs.join(",")))?;

    // TODO(akhercha): Build Pragma's signing key from AWS secret
    let pragma_signer = SigningKey::from_random();

    let mut response: SubscribeToEntryResponse = Default::default();
    for entry in median_entries {
        let median_price = entry.median_price.clone();
        let mut oracle_price: AssetOraclePrice = entry.into();

        // We need to sign (as Pragma) every computed median price
        let hash_to_sign = get_entry_hash(
            "Pragma",
            &oracle_price.global_asset_id,
            chrono::Utc::now().timestamp() as u64,
            &median_price,
        );
        let signature = pragma_signer
            .sign(&hash_to_sign)
            .map_err(EntryError::InvalidSigner)?;

        oracle_price.signature = signature.to_string();
        response.oracle_prices.push(oracle_price);
    }
    response.timestamp = chrono::Utc::now().timestamp().to_string();
    Ok(response)
}
