use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use pragma_entities::EntryError;
use serde::{Deserialize, Serialize};
use serde_json::json;
use starknet::core::utils::cairo_short_string_to_felt;
use starknet::signers::SigningKey;
use tokio::time::interval;

use crate::handlers::entries::SubscribeToEntryResponse;
use crate::infra::repositories::entry_repository::get_last_entries_for_pairs;
use crate::utils::{get_price_message, sign};
use crate::AppState;

use super::{SignedOraclePrice, StarkSignature, TimestampedSignature};

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
    ws.on_upgrade(move |socket| handle_subscription(socket, state))
}

async fn handle_subscription(mut socket: WebSocket, state: AppState) {
    let waiting_duration = Duration::from_millis(UPDATE_INTERVAL_IN_MS);
    // TODO(akhercha): Listen for changes in the entries dataase for subscribed pairs
    let mut update_interval = interval(waiting_duration);

    // OraclePricesTick response containing the past entries
    let mut entries = SubscribeToEntryResponse::default();

    // Pairs that the client is subscribed to
    let mut subscribed_pairs: Vec<String> = Vec::new();

    // TODO(akhercha): refinements for readability
    loop {
        tokio::select! {
            Some(msg) = socket.recv() => {
                if let Ok(Message::Text(text)) = msg {
                    // Handle subscription/unsubscription messages
                    if let Ok(subscription_msg) = serde_json::from_str::<SubscriptionRequest>(&text) {
                        // TODO(akhercha): what do we do about non existing pairs?
                        match subscription_msg.msg_type {
                            SubscriptionType::Subscribe => {
                                subscribed_pairs.extend(subscription_msg.pairs.clone());
                                subscribed_pairs.dedup();
                            },
                            SubscriptionType::Unsubscribe => {
                                subscribed_pairs.retain(|pair| !subscription_msg.pairs.contains(pair));
                            },
                        };
                        // Acknowledge subscription/unsubscription
                        let ack_message = serde_json::to_string(&SubscriptionAck {
                            msg_type: subscription_msg.msg_type,
                            pairs: subscribed_pairs.clone(),
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
                // Break channel if there's no more subscriptions
                if subscribed_pairs.is_empty() {
                    break;
                }
                // TODO(akhercha): Fetch entries for subscribed pairs
                match refresh_entries(&mut entries, &state, subscribed_pairs.clone()).await {
                    Ok(_) => {},
                    // Send error message and break channel if there's an error refreshing entries
                    Err(e) => {
                        socket.send(Message::Text(json!({ "error": e.to_string() }).to_string())).await.unwrap();
                        tracing::error!("Error refreshing entries: {:?}", e);
                        break;
                    }
                };
                let json_response = serde_json::to_string(&entries).unwrap();
                // Send entries to the client
                if socket.send(Message::Text(json_response)).await.is_err() {
                    // Stop channel if there is an error sending message
                    break;
                }
            }
        }
    }
}

async fn refresh_entries(
    entries: &mut SubscribeToEntryResponse,
    state: &AppState,
    subscribed_pairs: Vec<String>,
) -> Result<(), EntryError> {
    let last_entries = get_last_entries_for_pairs(&state.timescale_pool, subscribed_pairs).await?;

    for entry in last_entries {
        let pair_id_hex = cairo_short_string_to_felt(&entry.pair_id).unwrap();
        let pair_id_hex = format!("0x{:x}", pair_id_hex);

        // TODO(akhercha): Use existing interval median method with 500ms
        let asset_oracle_price = entries.oracle_prices.entry(pair_id_hex).or_default();

        // TODO(akhercha): Should be a median; not last price
        asset_oracle_price.price = entry.price.to_string();

        let (external_asset_id, hash_to_sign) = get_price_message(
            // TODO(akhercha): Store our Publisher name somewhere
            "Pragma",
            &entry.pair_id,
            entry.timestamp.and_utc().timestamp() as u64,
            &entry.price,
        );

        // TODO(akhercha): Wrong ATM - Sign every price with our registered StarkEx key when price published
        let signer = SigningKey::from_random();
        // TODO(akhercha): unsafe unwrap
        let signature = sign(&signer, hash_to_sign).unwrap();

        // TODO(akhercha): Wrong - should be all the prices used to compute the median
        let signed_price = SignedOraclePrice {
            price: entry.price.to_string(),
            timestamped_signature: TimestampedSignature {
                // TODO(akhercha): Bad, we should sign every price with our registered
                // starkex key when the price is published
                signature: StarkSignature {
                    r: format!("0x{}", signature.r.to_string()),
                    s: format!("0x{}", signature.s.to_string()),
                },
                timestamp: entry.timestamp.and_utc().timestamp().to_string(),
            },
            external_asset_id: format!("0x{}", external_asset_id),
        };

        // TODO(akhercha): Get our public key from somewhere
        let publisher_public_key = signer.verifying_key().scalar();
        let publisher_public_key_hex = format!("{:x}", publisher_public_key);

        asset_oracle_price
            .signed_prices
            .entry(publisher_public_key_hex)
            .or_insert_with(|| vec![signed_price.clone()])
            .insert(0, signed_price);
    }

    entries.timestamp = chrono::Utc::now().timestamp().to_string();

    Ok(())
}
