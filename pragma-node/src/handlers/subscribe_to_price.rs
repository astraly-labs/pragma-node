use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, State};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use pragma_common::types::timestamp::UnixTimestamp;
use pragma_entities::EntryError;
use utoipa::{ToResponse, ToSchema};

use crate::infra::repositories::entry_repository::get_price_with_components;
use crate::state::AppState;
use crate::utils::only_existing_pairs;
use crate::utils::ws::{ChannelHandler, Subscriber, SubscriptionType};

#[derive(Debug, Default, Serialize, Deserialize, ToResponse, ToSchema)]
pub struct AssetOraclePrice {
    num_sources_aggregated: usize,
    pair_id: String,
    price: String,
}

#[derive(Debug, Default, Serialize, Deserialize, ToResponse, ToSchema)]
pub struct SubscribeToPriceResponse {
    pub oracle_prices: Vec<AssetOraclePrice>,
    #[schema(value_type = i64)]
    pub timestamp: UnixTimestamp,
}

#[tracing::instrument(skip(state, ws), fields(endpoint_name = "subscribe_to_price"))]
pub async fn subscribe_to_price(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| create_new_subscriber(socket, state, client_addr))
}

/// Interval in milliseconds that the channel will update the client with the latest prices.
const CHANNEL_UPDATE_INTERVAL_IN_MS: u64 = 500;

#[tracing::instrument(
    skip(socket, app_state),
    fields(
        subscriber_id,
        client_ip = %client_addr.ip()
    )
)]
async fn create_new_subscriber(socket: WebSocket, app_state: AppState, client_addr: SocketAddr) {
    let mut subscriber = match Subscriber::<SubscriptionState>::new(
        "subscribe_to_price".into(),
        socket,
        client_addr.ip(),
        Arc::new(app_state),
        None,
        CHANNEL_UPDATE_INTERVAL_IN_MS,
        None,
    ) {
        Ok(subscriber) => subscriber,
        Err(e) => {
            tracing::error!("Failed to register subscriber: {}", e);
            return;
        }
    };

    // Main event loop for the subscriber
    let handler = WsEntriesHandler;
    let status = subscriber.listen(handler).await;
    if let Err(e) = status {
        tracing::error!(
            "[{}] Error occurred while listening to the subscriber: {:?}",
            subscriber.id,
            e
        );
    }
}

struct WsEntriesHandler;

#[async_trait::async_trait]
impl ChannelHandler<SubscriptionState, SubscriptionRequest, EntryError> for WsEntriesHandler {
    #[tracing::instrument(
        skip(self, subscriber),
        fields(
            subscriber_id = %subscriber.id,
            request_type = ?request.msg_type,
            pairs_count = request.pairs.len()
        )
    )]
    async fn handle_client_msg(
        &mut self,
        subscriber: &mut Subscriber<SubscriptionState>,
        request: SubscriptionRequest,
    ) -> Result<(), EntryError> {
        let (existing_spot_pairs, _existing_perp_pairs) =
            only_existing_pairs(&subscriber.app_state.offchain_pool, request.pairs).await;
        let mut state = subscriber.state.write().await;
        match request.msg_type {
            SubscriptionType::Subscribe => {
                state.add_spot_pairs(existing_spot_pairs);
            }
            SubscriptionType::Unsubscribe => {
                state.remove_spot_pairs(&existing_spot_pairs);
            }
        };
        let subscribed_pairs = state.get_subscribed_spot_pairs();
        drop(state);
        // We send an ack message to the client with the subscribed pairs (so
        // the client knows which pairs are successfully subscribed).
        let ack = SubscriptionAck {
            msg_type: request.msg_type,
            pairs: subscribed_pairs,
        };
        if let Err(e) = subscriber.send_msg(ack).await {
            let error_msg = format!("Message received but could not send ack message: {e}");
            subscriber.send_err(&error_msg).await;
        }
        Ok(())
    }

    #[tracing::instrument(
        skip(self, subscriber),
        fields(
            subscriber_id = %subscriber.id
        )
    )]
    async fn periodic_interval(
        &mut self,
        subscriber: &mut Subscriber<SubscriptionState>,
    ) -> Result<(), EntryError> {
        let subscription = subscriber.state.read().await;
        if subscription.is_empty() {
            return Ok(());
        }
        let response = match self
            .get_subscribed_pairs_medians(&subscriber.app_state, &subscription)
            .await
        {
            Ok(response) => response,
            Err(e) => {
                drop(subscription);
                subscriber.send_err(&e.to_string()).await;
                return Err(e);
            }
        };
        drop(subscription);
        if let Err(e) = subscriber.send_msg(response).await {
            subscriber
                .send_err(&format!("Could not send prices: {e}"))
                .await;
        }
        Ok(())
    }
}

impl WsEntriesHandler {
    /// Get the current median entries for the subscribed pairs and sign them as Pragma.
    #[tracing::instrument(
        skip(self, state, subscription),
        fields(
            subscribed_pairs = ?subscription.get_subscribed_spot_pairs().len()
        )
    )]
    async fn get_subscribed_pairs_medians(
        &self,
        state: &AppState,
        subscription: &SubscriptionState,
    ) -> Result<SubscribeToPriceResponse, EntryError> {
        let spot_pairs = subscription.get_subscribed_spot_pairs();
        if spot_pairs.is_empty() {
            return Ok(Default::default());
        }
        let median_entries = get_price_with_components(&state.offchain_pool, spot_pairs, false)
            .await
            .map_err(|e| EntryError::DatabaseError(format!("Failed to fetch price data: {e}")))?;

        // Convert HashMap entries to the expected response format
        let oracle_prices = median_entries
            .into_iter()
            .map(|(pair_id, entry)| AssetOraclePrice {
                num_sources_aggregated: entry.num_sources as usize,
                pair_id,
                price: entry.median_price.to_string(),
            })
            .collect();

        Ok(SubscribeToPriceResponse {
            timestamp: chrono::Utc::now().timestamp(),
            oracle_prices,
        })
    }
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
struct SubscriptionState {
    spot_pairs: HashSet<String>,
}

impl SubscriptionState {
    fn is_empty(&self) -> bool {
        self.spot_pairs.is_empty()
    }

    fn add_spot_pairs(&mut self, pairs: Vec<String>) {
        self.spot_pairs.extend(pairs);
    }

    fn remove_spot_pairs(&mut self, pairs: &[String]) {
        for pair in pairs {
            self.spot_pairs.remove(pair);
        }
    }

    /// Get the subscribed spot pairs.
    fn get_subscribed_spot_pairs(&self) -> Vec<String> {
        self.spot_pairs.iter().cloned().collect()
    }
}
