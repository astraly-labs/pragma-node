use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

use pragma_common::signing::sign_data;
use pragma_common::signing::starkex::StarkexPrice;
use pragma_common::types::DataType;
use pragma_common::types::timestamp::UnixTimestamp;
use pragma_entities::EntryError;

use crate::AppState;
use crate::constants::starkex_ws::PRAGMA_ORACLE_NAME_FOR_STARKEX;
use crate::infra::repositories::entry_repository::MedianEntryWithComponents;
use crate::utils::only_existing_pairs;
use crate::utils::pricer::{IndexPricer, MarkPricer, Pricer};
use crate::utils::{ChannelHandler, Subscriber, SubscriptionType};

/// Response format for StarkEx price subscriptions
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SignedPublisherPrice {
    /// StarkEx-specific asset identifier in hex format
    /// Format: <ASSET><CURRENCY>00..00PRAGMA00
    #[schema(example = "0x534f4c55534400000000000000000000505241474d4100")]
    pub oracle_asset_id: String,

    /// Price in StarkEx decimal format (not hex)
    /// The price is scaled by 10^18
    #[schema(example = "128065038090000000000")]
    pub oracle_price: String,

    /// Public key of the price signer (Pragma's StarkEx key)
    #[schema(example = "0x624EBFB99865079BD58CFCFB925B6F5CE940D6F6E41E118B8A72B7163FB435C")]
    pub signing_key: String,

    /// Unix timestamp as string
    #[schema(example = "1741594457")]
    pub timestamp: String,
}

/// Price data structure for StarkEx oracle integration
#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct AssetOraclePrice {
    /// Global asset identifier in StarkEx hex format
    /// Format: <ASSET>-<CURRENCY>-<DECIMALS>00..00
    #[schema(example = "0x534f4c2d5553442d38000000000000")]
    pub global_asset_id: String,

    /// Median price in StarkEx decimal format
    /// The price is scaled by 10^18
    #[schema(example = "128065038090000007168")]
    pub median_price: String,

    /// Pragma's signature of the price data in StarkEx format
    #[schema(
        example = "0x02ba39e956bb5b29a0fab31d61c7678228f79dddee2998b4ff3de5c7a6ae1e770636712af81b0506749555e1439004b4ce905419d2ba946b9bd06eb87de7a167"
    )]
    pub signature: String,

    /// Individual signed prices from publishers
    pub signed_prices: Vec<SignedPublisherPrice>,
}

/// WebSocket response message for StarkEx price updates
#[derive(Debug, Default, Serialize, Deserialize, ToResponse, ToSchema)]
#[schema(example = json!({
    "oracle_prices": [{
        "global_asset_id": "0x534f4c2d5553442d38000000000000",
        "median_price": "128065038090000007168",
        "signature": "0x02ba39e956bb5b29a0fab31d61c7678228f79dddee2998b4ff3de5c7a6ae1e770636712af81b0506749555e1439004b4ce905419d2ba946b9bd06eb87de7a167",
        "signed_prices": [{
            "oracle_asset_id": "0x534f4c55534400000000000000000000505241474d4100",
            "oracle_price": "128065038090000000000",
            "signing_key": "0x624EBFB99865079BD58CFCFB925B6F5CE940D6F6E41E118B8A72B7163FB435C",
            "timestamp": "1741594457"
        }]
    }],
    "timestamp": 1741594458
}))]
pub struct SubscribeToEntryResponse {
    /// Array of price data for subscribed assets
    pub oracle_prices: Vec<AssetOraclePrice>,

    /// Unix timestamp of the update
    #[schema(value_type = i64, example = 1741594458)]
    pub timestamp: UnixTimestamp,
}

#[utoipa::path(
    get,
    path = "/node/v1/data/subscribe",
    tag = "StarkEx Oracle",
    responses(
        (status = 101, description = "WebSocket connection upgraded successfully"),
        (status = 403, description = "Forbidden - Rate limit exceeded", body = EntryError),
        (status = 500, description = "Internal server error", body = EntryError,
         example = json!({"error": "Locked: Pragma signer not found"}))
    ),
)]
#[tracing::instrument(skip(state, ws), fields(endpoint_name = "subscribe_to_entry"))]
pub async fn subscribe_to_entry(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    if state.pragma_signer.is_none() {
        return (StatusCode::LOCKED, "Locked: Pragma signer not found").into_response();
    }
    ws.on_upgrade(move |socket| create_new_subscriber(socket, state, client_addr))
}

#[tracing::instrument(
    skip(socket, app_state),
    fields(
        subscriber_id,
        client_ip = %client_addr.ip()
    )
)]
async fn create_new_subscriber(socket: WebSocket, app_state: AppState, client_addr: SocketAddr) {
    /// Interval in milliseconds that the channel will update the client with the latest prices.
    const CHANNEL_UPDATE_INTERVAL_IN_MS: u64 = 500;

    let (mut subscriber, _) = match Subscriber::<SubscriptionState>::new(
        "subscribe_to_entry".into(),
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
            msg_type = ?request.msg_type,
            pairs = ?request.pairs
        )
    )]
    async fn handle_client_msg(
        &mut self,
        subscriber: &mut Subscriber<SubscriptionState>,
        request: SubscriptionRequest,
    ) -> Result<(), EntryError> {
        let (existing_spot_pairs, existing_perp_pairs) =
            only_existing_pairs(&subscriber.app_state.offchain_pool, request.pairs).await;
        let mut state = subscriber.state.lock().await;
        match request.msg_type {
            SubscriptionType::Subscribe => {
                state.add_spot_pairs(existing_spot_pairs);
                state.add_perp_pairs(existing_perp_pairs);
            }
            SubscriptionType::Unsubscribe => {
                state.remove_spot_pairs(&existing_spot_pairs);
                state.remove_perp_pairs(&existing_perp_pairs);
            }
        };
        let subscribed_pairs = state.get_fmt_subscribed_pairs();
        drop(state);
        // We send an ack message to the client with the subscribed pairs (so
        // the client knows which pairs are successfully subscribed).
        if let Ok(ack_message) = serde_json::to_string(&SubscriptionAck {
            msg_type: request.msg_type,
            pairs: subscribed_pairs,
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
        let subscription = subscriber.state.lock().await;
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
        if let Ok(json_response) = serde_json::to_string(&response) {
            if subscriber.send_msg(json_response).await.is_err() {
                subscriber.send_err("Could not send prices.").await;
            }
        } else {
            subscriber.send_err("Could not serialize prices.").await;
        }
        Ok(())
    }
}

impl WsEntriesHandler {
    /// Get the current median entries for the subscribed pairs and sign them as Pragma.
    #[tracing::instrument(
        skip(self, state, subscription),
        fields(
            spot_pairs = ?subscription.get_subscribed_spot_pairs().len(),
            perp_pairs = ?subscription.get_subscribed_perp_pairs().len()
        )
    )]
    async fn get_subscribed_pairs_medians(
        &self,
        state: &AppState,
        subscription: &SubscriptionState,
    ) -> Result<SubscribeToEntryResponse, EntryError> {
        let median_entries = self.get_all_entries(state, subscription).await?;

        let mut response: SubscribeToEntryResponse = Default::default();
        let now = chrono::Utc::now().timestamp();

        let pragma_signer = state
            .pragma_signer
            .as_ref()
            // Should not happen, as the endpoint is disabled if the signer is not found.
            .ok_or(EntryError::InternalServerError(
                "No Signer for Pragma".into(),
            ))?;

        for entry in median_entries {
            let pair_id = entry.pair_id.clone();

            let starkex_price = StarkexPrice {
                oracle_name: PRAGMA_ORACLE_NAME_FOR_STARKEX.to_string(),
                pair_id: pair_id.clone(),
                timestamp: now as u64,
                price: entry.median_price.clone(),
            };
            let signature =
                sign_data(pragma_signer, &starkex_price).map_err(|_| EntryError::InvalidSigner)?;

            // Create AssetOraclePrice with the original entry (it will be scaled in the TryFrom implementation)
            let mut oracle_price: AssetOraclePrice = entry.try_into().map_err(|_| {
                EntryError::InternalServerError("Could not create Oracle price".into())
            })?;
            oracle_price.signature = signature;
            response.oracle_prices.push(oracle_price);
        }
        response.timestamp = now;
        Ok(response)
    }

    /// Get index & mark prices for the subscribed pairs.
    #[tracing::instrument(skip(self, state, subscription))]
    async fn get_all_entries(
        &self,
        state: &AppState,
        subscription: &SubscriptionState,
    ) -> Result<Vec<MedianEntryWithComponents>, EntryError> {
        let index_pricer = IndexPricer::new(
            subscription.get_subscribed_spot_pairs(),
            DataType::SpotEntry,
        );

        let (usd_pairs, non_usd_pairs): (Vec<String>, Vec<String>) = subscription
            .get_subscribed_perp_pairs()
            .into_iter()
            .partition(|pair| {
                tracing::debug!("Checking pair for USD: {}", pair);
                pair.ends_with("USD")
            });
        tracing::debug!(
            "USD pairs: {:?}, non-USD pairs: {:?}",
            usd_pairs,
            non_usd_pairs
        );
        let mark_pricer_usd = IndexPricer::new(usd_pairs, DataType::PerpEntry);
        let mark_pricer_non_usd = MarkPricer::new(non_usd_pairs, DataType::PerpEntry);

        // Compute entries concurrently
        let (index_entries, usd_mark_entries, non_usd_mark_entries) = tokio::join!(
            index_pricer.compute(&state.offchain_pool),
            mark_pricer_usd.compute(&state.offchain_pool),
            mark_pricer_non_usd.compute(&state.offchain_pool)
        );

        let mut median_entries = vec![];
        median_entries.extend(index_entries.unwrap_or_default());

        // Add :MARK suffix to mark prices
        let mut usd_mark_entries = usd_mark_entries.unwrap_or_default();
        for entry in &mut usd_mark_entries {
            entry.pair_id = format!("{}:MARK", entry.pair_id);
        }
        median_entries.extend(usd_mark_entries);

        let mut non_usd_mark_entries = non_usd_mark_entries.unwrap_or_default();
        for entry in &mut non_usd_mark_entries {
            entry.pair_id = format!("{}:MARK", entry.pair_id);
        }
        median_entries.extend(non_usd_mark_entries);

        Ok(median_entries)
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
    perp_pairs: HashSet<String>,
}

impl SubscriptionState {
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
            .map(|pair| format!("{pair}:MARK"))
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
