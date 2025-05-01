use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, State};
use axum::response::IntoResponse;

use pragma_entities::InfraError;
use serde::{Deserialize, Serialize};

use pragma_common::Interval;
use utoipa::{ToResponse, ToSchema};

use crate::infra::repositories::entry_repository::{self, OHLCEntry};
use crate::state::AppState;
use crate::utils::{ChannelHandler, Subscriber, SubscriptionType};

use axum::extract::ws::{WebSocket, WebSocketUpgrade};

#[derive(Debug, Default, Serialize, Deserialize, ToSchema, ToResponse)]
pub struct GetOnchainOHLCResponse {
    pub pair_id: String,
    pub data: Vec<OHLCEntry>,
}

#[tracing::instrument(skip(state, ws), fields(endpoint_name = "subscribe_to_offchain_ohlc"))]
pub async fn subscribe_to_ohlc(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| create_new_subscriber(socket, state, client_addr))
}

/// Interval in milliseconds that the channel will update the client with the latest prices.
const CHANNEL_UPDATE_INTERVAL_IN_MS: u64 = 30000;

#[tracing::instrument(
    skip(socket, app_state),
    fields(
        subscriber_id,
        client_ip = %client_addr.ip()
    )
)]
async fn create_new_subscriber(socket: WebSocket, app_state: AppState, client_addr: SocketAddr) {
    let mut subscriber = match Subscriber::<SubscriptionState>::new(
        "subscribe_to_offchain_ohlc".into(),
        socket,
        client_addr.ip(),
        Arc::new(app_state),
        None,
        CHANNEL_UPDATE_INTERVAL_IN_MS,
        None,
    ) {
        Ok(subscriber) => subscriber,
        Err(e) => {
            tracing::error!("Failed to register subscriber: {:?}", e);
            return;
        }
    };

    // Main event loop for the subscriber
    let handler = WsOHLCHandler;
    if let Err(e) = subscriber.listen(handler).await {
        tracing::error!(
            "[{}] Error occurred while listening to the subscriber: {:?}",
            subscriber.id,
            e
        );
    }
}

struct WsOHLCHandler;

#[async_trait::async_trait]
impl ChannelHandler<SubscriptionState, SubscriptionRequest, InfraError> for WsOHLCHandler {
    #[tracing::instrument(
        skip(self, subscriber),
        fields(
            subscriber_id = %subscriber.id,
            pair = %subscription.pair,
            interval = ?subscription.interval
        )
    )]
    async fn handle_client_msg(
        &mut self,
        subscriber: &mut Subscriber<SubscriptionState>,
        subscription: SubscriptionRequest,
    ) -> Result<(), InfraError> {
        match subscription.msg_type {
            SubscriptionType::Subscribe => {
                let pair_exists = crate::utils::is_existing_pair(
                    &subscriber.app_state.offchain_pool,
                    &subscription.pair,
                )
                .await;
                if !pair_exists {
                    subscriber
                        .send_err("Pair does not exist in the onchain database.")
                        .await;
                    return Ok(());
                }
                let mut state = subscriber.state.write().await;
                *state = SubscriptionState {
                    subscribed_pair: Some(subscription.pair.clone()),
                    interval: subscription.interval,
                    is_first_update: true,
                    candles_to_get: subscription.candles_to_get.unwrap_or(10),
                };
            }
            SubscriptionType::Unsubscribe => {
                let mut state = subscriber.state.write().await;
                *state = SubscriptionState::default();
            }
        };
        self.send_ack_message(subscriber, subscription).await?;
        self.periodic_interval(subscriber).await?;
        Ok(())
    }

    #[tracing::instrument(
        skip(self, subscriber),
        fields(
            subscriber_id = %subscriber.id
        ),
        err(Debug)
    )]
    async fn periodic_interval(
        &mut self,
        subscriber: &mut Subscriber<SubscriptionState>,
    ) -> Result<(), InfraError> {
        let mut state = subscriber.state.write().await;
        if state.subscribed_pair.is_none() {
            return Ok(());
        }

        let ohlc_to_retrieve = if state.is_first_update {
            state.is_first_update = false;
            state.candles_to_get
        } else {
            1
        };
        let pair_id = state.subscribed_pair.clone().unwrap();

        let ohlc_data_res = entry_repository::get_spot_ohlc(
            &subscriber.app_state.onchain_pool,
            pair_id.clone(),
            state.interval,
            ohlc_to_retrieve as i64,
        )
        .await;
        drop(state);

        if let Err(e) = ohlc_data_res {
            subscriber.send_err(&e.to_string()).await;
            return Err(e);
        }

        if subscriber.send_msg(ohlc_data_res.unwrap()).await.is_err() {
            return Err(InfraError::InternalServerError);
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
        let ack_message = SubscriptionAck {
            msg_type: subscription.msg_type,
            pair: subscription.pair,
            interval: subscription.interval,
        };

        if subscriber.send_msg(ack_message).await.is_err() {
            subscriber
                .send_err("Message received but could not send ack message.")
                .await;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct SubscriptionState {
    subscribed_pair: Option<String>,
    interval: Interval,
    is_first_update: bool,
    candles_to_get: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct SubscriptionRequest {
    msg_type: SubscriptionType,
    pair: String,
    interval: Interval,
    candles_to_get: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SubscriptionAck {
    msg_type: SubscriptionType,
    pair: String,
    interval: Interval,
}
