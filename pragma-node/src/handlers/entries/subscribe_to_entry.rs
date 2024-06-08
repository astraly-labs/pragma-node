use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use bigdecimal::BigDecimal;
use pragma_entities::EntryError;
use serde::{Deserialize, Serialize};
use serde_json::json;
use starknet::signers::SigningKey;
use tokio::time::interval;

use crate::handlers::entries::SubscribeToEntryResponse;
use crate::infra::repositories::entry_repository::{
    get_current_median_entries_with_components, EntryComponent, MedianEntryWithComponents,
};
use crate::utils::{get_entry_hash, get_external_asset_id};
use crate::AppState;

use super::{AssetOraclePrice, SignedPublisherPrice};

const CHANNEL_UPDATE_INTERVAL_IN_MS: u64 = 500;

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
    let waiting_duration = Duration::from_millis(CHANNEL_UPDATE_INTERVAL_IN_MS);
    let mut update_interval = interval(waiting_duration);
    let mut subscribed_pairs: Vec<String> = Vec::new();

    loop {
        tokio::select! {
            Some(msg) = socket.recv() => {
                if let Ok(Message::Text(text)) = msg {
                    handle_message_received(&mut socket, &mut subscribed_pairs, text).await;
                }
            },
            _ = update_interval.tick() => {
                match send_median_entries(&mut socket, &state, &subscribed_pairs).await {
                    Ok(_) => {},
                    Err(_) => break
                };
            }
        }
    }
}

async fn handle_message_received(
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

        if let Ok(ack_message) = serde_json::to_string(&SubscriptionAck {
            msg_type: subscription_msg.msg_type,
            pairs: subscribed_pairs.clone(),
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

async fn send_median_entries(
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

async fn get_subscribed_pairs_entries(
    state: &AppState,
    subscribed_pairs: &[String],
) -> Result<SubscribeToEntryResponse, EntryError> {
    let median_entries =
        get_current_median_entries_with_components(&state.timescale_pool, subscribed_pairs)
            .await
            .map_err(|e| e.to_entry_error(&subscribed_pairs.join(",")))?;

    let pragma_signer = &state.pragma_signer;

    let mut response: SubscribeToEntryResponse = Default::default();
    for entry in median_entries {
        let median_price = entry.median_price.clone();
        let mut oracle_price: AssetOraclePrice = entry.into();

        let signature = sign_median_price_as_pragma(
            pragma_signer,
            &oracle_price.global_asset_id,
            median_price,
        )?;

        oracle_price.signature = signature;
        response.oracle_prices.push(oracle_price);
    }
    response.timestamp = chrono::Utc::now().timestamp().to_string();
    Ok(response)
}

/// Sign the median price as Pragma and return the signature
/// 0x prefixed.
fn sign_median_price_as_pragma(
    signer: &SigningKey,
    asset_id: &str,
    median_price: BigDecimal,
) -> Result<String, EntryError> {
    let oracle_name = "PRAGMA";
    let hash_to_sign = get_entry_hash(
        oracle_name,
        asset_id,
        chrono::Utc::now().timestamp() as u64,
        &median_price,
    );
    let signature = signer
        .sign(&hash_to_sign)
        .map_err(EntryError::InvalidSigner)?;
    Ok(format!("0x{:}", signature))
}

impl From<EntryComponent> for SignedPublisherPrice {
    fn from(component: EntryComponent) -> Self {
        let asset_id = get_external_asset_id(&component.publisher, &component.pair_id);
        SignedPublisherPrice {
            oracle_asset_id: format!("0x{}", asset_id),
            oracle_price: component.price.to_string(),
            timestamp: component.timestamp.to_string(),
            signing_key: component.publisher_address,
            signature: component.publisher_signature,
        }
    }
}

impl From<MedianEntryWithComponents> for AssetOraclePrice {
    fn from(median_entry: MedianEntryWithComponents) -> Self {
        AssetOraclePrice {
            global_asset_id: median_entry.pair_id,
            median_price: median_entry.median_price.to_string(),
            signed_prices: median_entry
                .components
                .into_iter()
                .map(SignedPublisherPrice::from)
                .collect(),
            signature: Default::default(),
        }
    }
}

async fn send_error_message(socket: &mut WebSocket, error: &str) {
    let error_msg = json!({ "error": error }).to_string();
    socket.send(Message::Text(error_msg)).await.unwrap();
}
