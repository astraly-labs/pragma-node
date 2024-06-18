use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, State};
use axum::response::IntoResponse;
use pragma_entities::InfraError;
use serde::{Deserialize, Serialize};

use pragma_common::types::{Interval, Network};

use crate::infra::repositories::entry_repository::OHLCEntry;
use crate::infra::repositories::onchain_repository::get_ohlc;
use crate::types::ws::{ChannelHandler, Subscriber, SubscriptionType};
use crate::utils::is_onchain_existing_pair;
use crate::AppState;

use axum::extract::ws::{WebSocket, WebSocketUpgrade};

#[utoipa::path(
    get,
    path = "/node/v1/onchain/ohlc/subscribe",
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
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| create_new_subscriber(socket, state, client_addr))
}

/// Interval in milliseconds that the channel will update the client with the latest prices.
const CHANNEL_UPDATE_INTERVAL_IN_MS: u64 = 1000; // 1 second

async fn create_new_subscriber(socket: WebSocket, app_state: AppState, client_addr: SocketAddr) {
    let (mut subscriber, _) = match Subscriber::<SubscriptionState>::new(
        socket,
        client_addr.ip(),
        Arc::new(app_state),
        None,
        CHANNEL_UPDATE_INTERVAL_IN_MS,
    )
    .await
    {
        Ok(subscriber) => subscriber,
        Err(e) => {
            tracing::error!("Failed to register subscriber: {}", e);
            return;
        }
    };

    // Main event loop for the subscriber
    let handler = WsOHLCHandler;
    let status = subscriber.listen(handler).await;
    if let Err(e) = status {
        tracing::error!(
            "[{}] Error occurred while listening to the subscriber: {:?}",
            subscriber.id,
            e
        );
    }
}

struct WsOHLCHandler;

impl ChannelHandler<SubscriptionState, SubscriptionRequest, InfraError> for WsOHLCHandler {
    async fn handle_client_msg(
        &mut self,
        subscriber: &mut Subscriber<SubscriptionState>,
        subscription: SubscriptionRequest,
    ) -> Result<(), InfraError> {
        match subscription.msg_type {
            SubscriptionType::Subscribe => {
                let pair_exists = is_onchain_existing_pair(
                    &subscriber.app_state.onchain_pool,
                    &subscription.pair,
                    subscription.network,
                )
                .await;
                if !pair_exists {
                    let error_msg = "Pair does not exist in the onchain database.";
                    subscriber.send_err(error_msg).await;
                    return Ok(());
                }
                let mut state = subscriber.state.lock().await;
                *state = SubscriptionState {
                    subscribed_pair: Some(subscription.pair.clone()),
                    network: subscription.network,
                    interval: subscription.interval,
                    is_first_update: true,
                    ohlc_data: Vec::new(),
                };
            }
            SubscriptionType::Unsubscribe => {
                let mut state = subscriber.state.lock().await;
                *state = SubscriptionState::default();
            }
        };
        self.send_ack_message(subscriber, subscription).await?;
        // Trigger the first update manually
        self.periodic_interval(subscriber).await?;
        Ok(())
    }

    async fn periodic_interval(
        &mut self,
        subscriber: &mut Subscriber<SubscriptionState>,
    ) -> Result<(), InfraError> {
        let mut state = subscriber.state.lock().await;
        if state.subscribed_pair.is_none() {
            return Ok(());
        }

        let ohlc_to_compute = if state.is_first_update {
            state.is_first_update = false;
            10
        } else {
            1
        };
        let pair_id = state.subscribed_pair.clone().unwrap();
        let mut ohlc_data = state.ohlc_data.clone();

        let status = get_ohlc(
            &mut ohlc_data,
            &subscriber.app_state.onchain_pool,
            state.network,
            pair_id.clone(),
            state.interval,
            ohlc_to_compute,
        )
        .await;

        state.ohlc_data.clone_from(&ohlc_data);
        drop(state);

        if let Err(e) = status {
            subscriber.send_err(&e.to_string()).await;
            return Err(e);
        }

        match serde_json::to_string(&ohlc_data) {
            Ok(json_response) => {
                if subscriber.send_msg(json_response).await.is_err() {
                    subscriber.send_err("Could not send prices.").await;
                }
            }
            Err(_) => {
                subscriber.send_err("Could not serialize prices.").await;
            }
        }

        Ok(())
    }
}

impl WsOHLCHandler {
    async fn send_ack_message(
        &self,
        subscriber: &mut Subscriber<SubscriptionState>,
        subscription: SubscriptionRequest,
    ) -> Result<(), InfraError> {
        if let Ok(ack_message) = serde_json::to_string(&SubscriptionAck {
            msg_type: subscription.msg_type,
            pair: subscription.pair,
            network: subscription.network,
            interval: subscription.interval,
        }) {
            if subscriber.send_msg(ack_message).await.is_err() {
                let error_msg = "Message received but could not send ack message.";
                subscriber.send_err(error_msg).await;
            }
        } else {
            let error_msg = "Could not serialize ack message.";
            subscriber.send_err(error_msg).await;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct SubscriptionState {
    subscribed_pair: Option<String>,
    network: Network,
    interval: Interval,
    is_first_update: bool,
    ohlc_data: Vec<OHLCEntry>,
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
