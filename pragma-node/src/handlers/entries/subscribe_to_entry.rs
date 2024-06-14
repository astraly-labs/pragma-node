use std::collections::HashSet;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;

use pragma_common::types::DataType;
use pragma_entities::EntryError;
use serde::{Deserialize, Serialize};
use tokio::time::interval;

use crate::handlers::entries::utils::send_err_to_socket;
use crate::handlers::entries::SubscribeToEntryResponse;
use crate::infra::repositories::entry_repository::MedianEntryWithComponents;
use crate::utils::pricing::{IndexPricer, MarkPricer, Pricer};
use crate::utils::sign_median_price;
use crate::AppState;

use super::utils::only_existing_pairs;
use super::AssetOraclePrice;

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

#[derive(Debug, Default, Serialize, Deserialize)]
struct CurrentSubscription {
    spot_pairs: HashSet<String>,
    perp_pairs: HashSet<String>,
}

impl CurrentSubscription {
    fn is_empty(&self) -> bool {
        self.spot_pairs.is_empty() && self.perp_pairs.is_empty()
    }

    fn add_spot_pairs(&mut self, pairs: Vec<String>) {
        self.spot_pairs.extend(pairs);
    }

    fn add_perp_pairs(&mut self, pairs: Vec<String>) {
        self.perp_pairs.extend(pairs);
    }

    fn remove_spot_pairs(&mut self, pairs: &[String]) {
        for pair in pairs {
            self.spot_pairs.remove(pair);
        }
    }

    fn remove_perp_pairs(&mut self, pairs: &[String]) {
        for pair in pairs {
            self.perp_pairs.remove(pair);
        }
    }

    /// Get the subscribed spot pairs.
    fn get_subscribed_spot_pairs(&self) -> Vec<String> {
        self.spot_pairs.iter().cloned().collect()
    }

    /// Get the subscribed perps pairs (without suffix).
    fn get_subscribed_perp_pairs(&self) -> Vec<String> {
        self.perp_pairs.iter().cloned().collect()
    }

    /// Get the subscribed perps pairs with the MARK suffix.
    fn get_fmt_subscribed_perp_pairs(&self) -> Vec<String> {
        self.perp_pairs
            .iter()
            .map(|pair| format!("{}:MARK", pair))
            .collect()
    }

    /// Get all the currently subscribed pairs.
    /// (Spot and Perp pairs with the suffix)
    fn get_fmt_subscribed_pairs(&self) -> Vec<String> {
        let mut spot_pairs = self.get_subscribed_spot_pairs();
        let perp_pairs = self.get_fmt_subscribed_perp_pairs();
        spot_pairs.extend(perp_pairs);
        spot_pairs
    }
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

/// Interval in milliseconds that the channel will update the client with the latest prices.
const CHANNEL_UPDATE_INTERVAL_IN_MS: u64 = 500;

/// Handle the WebSocket channel.
async fn handle_channel(mut socket: WebSocket, state: AppState) {
    let waiting_duration = Duration::from_millis(CHANNEL_UPDATE_INTERVAL_IN_MS);
    let mut update_interval = interval(waiting_duration);
    let mut subscription: CurrentSubscription = Default::default();

    loop {
        tokio::select! {
            Some(msg) = socket.recv() => {
                if let Ok(Message::Text(text)) = msg {
                    handle_message_received(&mut socket, &state, &mut subscription, text).await;
                }
            },
            _ = update_interval.tick() => {
                match send_median_entries(&mut socket, &state, &subscription).await {
                    Ok(_) => {},
                    Err(_) => break
                };
            },
        }
    }
}

/// Handle the message received from the client.
/// Subscribe or unsubscribe to the pairs requested.
async fn handle_message_received(
    socket: &mut WebSocket,
    state: &AppState,
    subscription: &mut CurrentSubscription,
    message: String,
) {
    if let Ok(subscription_msg) = serde_json::from_str::<SubscriptionRequest>(&message) {
        let (existing_spot_pairs, existing_perp_pairs) =
            only_existing_pairs(&state.timescale_pool, subscription_msg.pairs).await;
        match subscription_msg.msg_type {
            SubscriptionType::Subscribe => {
                subscription.add_spot_pairs(existing_spot_pairs);
                subscription.add_perp_pairs(existing_perp_pairs);
            }
            SubscriptionType::Unsubscribe => {
                subscription.remove_spot_pairs(&existing_spot_pairs);
                subscription.remove_perp_pairs(&existing_perp_pairs);
            }
        };
        // We send an ack message to the client with the subscribed pairs (so
        // the client knows which pairs are successfully subscribed).
        if let Ok(ack_message) = serde_json::to_string(&SubscriptionAck {
            msg_type: subscription_msg.msg_type,
            pairs: subscription.get_fmt_subscribed_pairs(),
        }) {
            if socket.send(Message::Text(ack_message)).await.is_err() {
                let error_msg = "Message received but could not send ack message.";
                send_err_to_socket(socket, error_msg).await;
            }
        } else {
            let error_msg = "Could not serialize ack message.";
            send_err_to_socket(socket, error_msg).await;
        }
    } else {
        let error_msg = "Invalid message type. Please check the documentation for more info.";
        send_err_to_socket(socket, error_msg).await;
    }
}

/// Send the current median entries to the client.
async fn send_median_entries(
    socket: &mut WebSocket,
    state: &AppState,
    subscription: &CurrentSubscription,
) -> Result<(), EntryError> {
    if subscription.is_empty() {
        return Ok(());
    }
    let response = match get_subscribed_pairs_medians(state, subscription).await {
        Ok(response) => response,
        Err(e) => {
            send_err_to_socket(socket, &e.to_string()).await;
            return Err(e);
        }
    };

    if let Ok(json_response) = serde_json::to_string(&response) {
        if socket.send(Message::Text(json_response)).await.is_err() {
            send_err_to_socket(socket, "Could not send prices.").await;
        }
    } else {
        send_err_to_socket(socket, "Could not serialize prices.").await;
    }
    Ok(())
}

/// Get the current median entries for the subscribed pairs and sign them as Pragma.
async fn get_subscribed_pairs_medians(
    state: &AppState,
    subscription: &CurrentSubscription,
) -> Result<SubscribeToEntryResponse, EntryError> {
    let median_entries = get_all_entries(state, subscription).await?;

    let mut response: SubscribeToEntryResponse = Default::default();
    let now = chrono::Utc::now().timestamp();

    for entry in median_entries {
        let median_price = entry.median_price.clone();
        let pair_id = entry.pair_id.clone();
        let mut oracle_price: AssetOraclePrice = entry
            .try_into()
            .map_err(|_| EntryError::InternalServerError)?;

        let signature = sign_median_price(&state.pragma_signer, &pair_id, now as u64, median_price)
            .map_err(|_| EntryError::InvalidSigner)?;

        oracle_price.signature = signature;
        response.oracle_prices.push(oracle_price);
    }
    response.timestamp_s = now;
    Ok(response)
}

/// Get index & mark prices for the subscribed pairs.
async fn get_all_entries(
    state: &AppState,
    subscription: &CurrentSubscription,
) -> Result<Vec<MedianEntryWithComponents>, EntryError> {
    let index_pricer = IndexPricer::new(
        subscription.get_subscribed_spot_pairs(),
        DataType::SpotEntry,
    );

    let (usd_pairs, non_usd_pairs): (Vec<String>, Vec<String>) = subscription
        .get_subscribed_perp_pairs()
        .into_iter()
        .partition(|pair| pair.ends_with("USD"));
    let mark_pricer_usd = IndexPricer::new(usd_pairs, DataType::PerpEntry);
    let mark_pricer_non_usd = MarkPricer::new(non_usd_pairs, DataType::PerpEntry);

    let (index_entries, usd_mark_entries, non_usd_mark_entries) = tokio::join!(
        index_pricer.compute(&state.timescale_pool),
        mark_pricer_usd.compute(&state.timescale_pool),
        mark_pricer_non_usd.compute(&state.timescale_pool)
    );

    let mut median_entries = vec![];
    median_entries.extend(index_entries.unwrap_or_default());
    median_entries.extend(usd_mark_entries.unwrap_or_default());
    median_entries.extend(non_usd_mark_entries.unwrap_or_default());
    Ok(median_entries)
}
