use std::fmt::Debug;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, State};
use axum::response::IntoResponse;
use futures_util::SinkExt;
use serde::{Deserialize, Serialize};

use crate::types::ws::{ChannelHandler, Subscriber};
use crate::AppState;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WsState {
    pub count: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ClientMsg {
    pub msg: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ServerMsg {
    pub msg: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PriceUpdate {
    new_price: String,
}

#[derive(thiserror::Error, Debug)]
pub enum TestError {
    #[error("Example error")]
    ExampleError,
}

struct WsTestHandler;
impl ChannelHandler<WsState, ClientMsg, ServerMsg, TestError> for WsTestHandler {
    async fn handle_client_msg(
        &mut self,
        _subscriber: &mut Subscriber<WsState>,
        message: ClientMsg,
    ) -> Result<(), TestError> {
        tracing::info!("{:?}", message);
        Ok(())
    }

    async fn handle_server_msg(
        &mut self,
        subscriber: &mut Subscriber<WsState>,
        message: ServerMsg,
    ) -> Result<(), TestError> {
        let _ = subscriber
            .sender
            .send(Message::Text(serde_json::to_string(&message).unwrap()))
            .await;
        Ok(())
    }

    async fn periodic_interval(
        &mut self,
        subscriber: &mut Subscriber<WsState>,
    ) -> Result<(), TestError> {
        if subscriber.closed {
            return Ok(());
        }

        subscriber.state.count += 1;
        if subscriber.state.count > 10 {
            return Err(TestError::ExampleError);
        }

        if let Ok(msg) = serde_json::to_string(&PriceUpdate {
            new_price: "100".to_string(),
        }) {
            let _ = subscriber.sender.send(Message::Text(msg)).await;
        }
        Ok(())
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
    let (mut subscriber, _) =
        match Subscriber::<WsState>::new(socket, client_addr.ip(), Arc::new(app_state), 1000).await
        {
            Ok(subscriber) => subscriber,
            Err(e) => {
                tracing::error!("Failed to create a new subscriber. Error: {}", e);
                return;
            }
        };

    // Main event loop for the subscriber
    let handler = WsTestHandler;
    let status = subscriber.listen(handler).await;
    if let Err(e) = status {
        tracing::error!("Error occurred while listening to the subscriber: {:?}", e);
    }
}
