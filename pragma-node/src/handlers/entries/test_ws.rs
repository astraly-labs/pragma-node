use std::fmt::Debug;
use std::net::SocketAddr;
use std::ops::ControlFlow;
use std::sync::Arc;
use tokio::sync::mpsc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, State};
use axum::response::IntoResponse;
use futures_util::SinkExt;
use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use crate::types::ws::{ChannelHandler, Subscriber};
use crate::AppState;

struct WsTestHandler;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WsState {
    pub msg: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ChannelUpdateMsg {
    pub msg: String,
}

impl ChannelHandler<WsState, ChannelUpdateMsg> for WsTestHandler {
    async fn handle_client_message(&mut self, message: ChannelUpdateMsg) {
        tracing::info!("{:?}", message);
    }

    async fn handle_server_message(
        &mut self,
        subscriber: &mut Subscriber<WsState, ChannelUpdateMsg>,
        message: ChannelUpdateMsg,
    ) {
        let _ = subscriber
            .sender
            .send(Message::Text(serde_json::to_string(&message).unwrap()))
            .await;
    }

    // Decode & handle messages received from the client
    async fn decode_and_handle_message_received(
        &mut self,
        subscriber: &mut Subscriber<WsState, ChannelUpdateMsg>,
        msg: Message,
    ) -> ControlFlow<(), ()> {
        match msg {
            Message::Close(_) => {
                tracing::info!("👋 [CLOSE]");
                subscriber.closed = true;
                return ControlFlow::Break(());
            }
            Message::Text(text) => {
                tracing::info!("📨 [TEXT]");
                let msg = serde_json::from_str::<ChannelUpdateMsg>(&text);
                if let Ok(msg) = msg {
                    self.handle_client_message(msg).await;
                } else {
                    tracing::error!("Could not decode message");
                }
            }
            Message::Binary(_) => {
                tracing::info!("📨 [BINARY]");
            }
            Message::Ping(_) => {
                tracing::info!("📨 [PING]");
            }
            Message::Pong(_) => {
                tracing::info!("📨 [PONG]");
            }
        }
        ControlFlow::Continue(())
    }

    async fn periodic_interval(&mut self, subscriber: &mut Subscriber<WsState, ChannelUpdateMsg>) {
        if subscriber.closed {
            return;
        }
        let _ = subscriber
            .sender
            .send(Message::Text("tic".to_string()))
            .await;
    }
}

#[utoipa::path(
    get,
    path = "/node/v1/data/test",
    responses(
        (
            status = 200,
            description = "Subscribe to a list of entries",
            body = [SubscribeToEntryResponse]
        )
    )
)]
pub async fn test_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| create_new_subscriber(socket, state, client_addr))
}

async fn create_new_subscriber(socket: WebSocket, app_state: AppState, client_addr: SocketAddr) {
    // Channel communication between the server & the subscriber
    let (_notify_sender, notify_receiver) = mpsc::channel::<ChannelUpdateMsg>(100);
    // Exit signal, allow to close the channel from the server side
    let (_exit_sender, exit_receiver) = watch::channel(false);

    let mut subscriber = match Subscriber::<WsState, ChannelUpdateMsg>::new(
        socket,
        client_addr.ip(),
        Arc::new(app_state),
        1000,
        notify_receiver,
        exit_receiver,
    )
    .await
    {
        Ok(subscriber) => subscriber,
        Err(e) => {
            tracing::error!("Failed to create a new subscriber. Error: {}", e);
            return;
        }
    };

    // Send a welcome message
    let _ = subscriber
        .sender
        .send(Message::Text(format!(
            "You are registered as:\n[{:?}] {}",
            subscriber.ip_address, subscriber.id
        )))
        .await;

    // Main event loop for the subscriber
    let handler = WsTestHandler;
    subscriber.listen(handler).await;
}
