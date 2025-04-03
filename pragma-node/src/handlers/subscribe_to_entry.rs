use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use pragma_common::errors::ConversionError;
use serde::{Deserialize, Serialize};
use starknet::signers::SigningKey;
use utoipa::{ToResponse, ToSchema};

use pragma_common::signing::sign_data;
use pragma_common::signing::starkex::StarkexPrice;
use pragma_common::types::timestamp::UnixTimestamp;
use pragma_entities::EntryError;

use crate::constants::starkex_ws::PRAGMA_ORACLE_NAME_FOR_STARKEX;
use crate::infra::repositories::entry_repository::{MedianEntry, get_price_with_components};
use crate::state::AppState;
use crate::utils::{ChannelHandler, Subscriber, SubscriptionType};
use crate::utils::{hex_string_to_bigdecimal, only_existing_pairs};

/// Response format for `StarkEx` price subscriptions
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SignedPublisherPrice {
    /// StarkEx-specific asset identifier in hex format
    /// Format: <ASSET><CURRENCY>00..00PRAGMA00
    #[schema(example = "0x534f4c55534400000000000000000000505241474d4100")]
    pub oracle_asset_id: String,

    /// Price in `StarkEx` 18 decimals
    #[schema(example = "128065038090000000000")]
    pub oracle_price: String,

    /// Public key of the price signer (Pragma's `StarkEx` key)
    #[schema(example = "0x624EBFB99865079BD58CFCFB925B6F5CE940D6F6E41E118B8A72B7163FB435C")]
    pub signing_key: String,

    /// Unix timestamp as string
    #[schema(example = "1741594457")]
    pub timestamp: String,

    // Signature
    #[schema(
        example = "0x03ac186cbbb633b7eb13e5a1c22454da7a6e9f6a4b81236380ec4564634afc30638641c0f2a613edc947241a68c54f0da4f08bc9dbb3b79154c87ad3d74ad83"
    )]
    pub signature: String,
}

/// Price data structure for `StarkEx` oracle integration
#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct AssetOraclePrice {
    /// Global asset identifier in `StarkEx` hex format
    /// Format: <ASSET>-<CURRENCY>-<DECIMALS>00..00
    #[schema(example = "0x534f4c2d5553442d38000000000000")]
    pub global_asset_id: String,

    /// Median price in `StarkEx` 18 decimals format
    #[schema(example = "128065038090000007168")]
    pub median_price: String,

    /// Pragma's signature of the price data in `StarkEx` format
    #[schema(
        example = "0x02ba39e956bb5b29a0fab31d61c7678228f79dddee2998b4ff3de5c7a6ae1e770636712af81b0506749555e1439004b4ce905419d2ba946b9bd06eb87de7a167"
    )]
    pub signature: String,

    /// Individual signed prices from publishers
    pub signed_prices: Vec<SignedPublisherPrice>,
}

/// WebSocket response message for `StarkEx` price updates
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
    "timestamp": 1_741_594_458
}))]
pub struct SubscribeToEntryResponse {
    /// Array of price data for subscribed assets
    pub oracle_prices: Vec<AssetOraclePrice>,

    /// Unix timestamp of the update
    #[schema(value_type = i64, example = 1_741_594_458)]
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
    const CHANNEL_UPDATE_INTERVAL_IN_MS: u64 = 100;

    let mut subscriber = match Subscriber::<SubscriptionState>::new(
        "subscribe_to_entry".into(),
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
        let mut state = subscriber.state.write().await;
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
            spot_pairs = ?subscription.get_subscribed_spot_pairs().len(),
            perp_pairs = ?subscription.get_subscribed_perp_pairs().len()
        )
    )]
    async fn get_subscribed_pairs_medians(
        &self,
        state: &AppState,
        subscription: &SubscriptionState,
    ) -> Result<SubscribeToEntryResponse, EntryError> {
        let spot_pairs = subscription.get_subscribed_spot_pairs();
        let perp_pairs = subscription.get_subscribed_perp_pairs();
        let number_of_spot_pairs = spot_pairs.len();
        let number_of_perp_pairs = perp_pairs.len();

        if number_of_spot_pairs == 0 && number_of_perp_pairs == 0 {
            return Err(EntryError::NoSubscribedPairs(
                "No pairs provided for subscription".into(),
            ));
        }

        let mut all_entries = if number_of_spot_pairs == 0 {
            HashMap::new()
        } else {
            let entries = get_price_with_components(&state.offchain_pool, spot_pairs, false)
                .await
                .map_err(|e| {
                    EntryError::DatabaseError(format!("Failed to fetch spot data: {e}"))
                })?;
            // Check if we got entries for all requested spot pairs
            if entries.len() < number_of_spot_pairs {
                tracing::debug!(
                    "Missing spot prices for some pairs. Found {} of {} requested pairs.",
                    entries.len(),
                    number_of_spot_pairs
                );
            }
            entries
        };

        if number_of_perp_pairs != 0 {
            let perp_entries = get_price_with_components(&state.offchain_pool, perp_pairs, true)
                .await
                .map_err(|e| {
                    EntryError::DatabaseError(format!("Failed to fetch perp data: {e}"))
                })?;
            // Check if we got entries for all requested perp pairs
            if perp_entries.len() < number_of_perp_pairs {
                tracing::debug!(
                    "Missing perp prices for some pairs. Found {} of {} requested pairs.",
                    perp_entries.len(),
                    number_of_perp_pairs
                );
            }
            // Merge the results
            all_entries.extend(perp_entries);
        }

        let mut response: SubscribeToEntryResponse = Default::default();
        let now = chrono::Utc::now().timestamp_millis();

        let pragma_signer = state
            .pragma_signer
            .as_ref()
            // Should not happen, as the endpoint is disabled if the signer is not found.
            .ok_or(EntryError::InternalServerError(
                "No Signer for Pragma".into(),
            ))?;

        for (pair_id, entry) in all_entries {
            let starkex_price = StarkexPrice {
                oracle_name: PRAGMA_ORACLE_NAME_FOR_STARKEX.to_string(),
                pair_id: pair_id.clone(),
                timestamp: now as u64,
                price: entry.median_price.clone(),
            };
            let signature =
                sign_data(pragma_signer, &starkex_price).map_err(|_| EntryError::InvalidSigner)?;

            let mut oracle_price: AssetOraclePrice =
                AssetOraclePrice::try_from((pair_id, entry, pragma_signer.clone())) // TODO: remove clone
                    .map_err(|_| {
                        EntryError::InternalServerError("Could not create Oracle price".into())
                    })?;
            oracle_price.signature = signature;
            response.oracle_prices.push(oracle_price);
        }
        response.timestamp = now;
        Ok(response)
    }
}

impl TryFrom<(String, MedianEntry, SigningKey)> for AssetOraclePrice {
    type Error = ConversionError;

    fn try_from(value: (String, MedianEntry, SigningKey)) -> Result<Self, Self::Error> {
        let (pair_id, entry, signing_key) = value;

        // Computes IDs
        let global_asset_id = StarkexPrice::get_global_asset_id(&pair_id)?;
        let oracle_asset_id =
            StarkexPrice::get_oracle_asset_id(PRAGMA_ORACLE_NAME_FOR_STARKEX, &pair_id)?;

        let signed_prices_result: Result<Vec<_>, ConversionError> = entry
            .components
            .unwrap_or_default()
            .into_iter()
            .map(|comp| {
                let timestamp = comp.timestamp.and_utc().timestamp_millis() as u64;
                let price = hex_string_to_bigdecimal(&comp.price)
                    .map_err(|_| ConversionError::StringPriceConversion)?;
                let starkex_price = StarkexPrice {
                    oracle_name: PRAGMA_ORACLE_NAME_FOR_STARKEX.to_string(),
                    pair_id: pair_id.clone(),
                    timestamp,
                    price: price.clone(),
                };
                let signature = sign_data(&signing_key, &starkex_price)
                    .map_err(|_| ConversionError::FailedSignature(pair_id.clone()))?;
                Ok(SignedPublisherPrice {
                    oracle_asset_id: format!("0x{oracle_asset_id}"),
                    oracle_price: price.to_string(),
                    signing_key: signing_key.secret_scalar().to_hex_string(),
                    timestamp: timestamp.to_string(),
                    signature,
                })
            })
            .collect();
        let signed_prices = signed_prices_result?;

        Ok(Self {
            global_asset_id: format!("0x{global_asset_id}"),
            median_price: entry.median_price.to_string(),
            signature: String::new(),
            signed_prices,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubscriptionRequest {
    pub msg_type: SubscriptionType,
    pub pairs: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubscriptionAck {
    pub msg_type: SubscriptionType,
    pub pairs: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SubscriptionState {
    pub spot_pairs: HashSet<String>,
    pub perp_pairs: HashSet<String>,
}

impl SubscriptionState {
    pub fn is_empty(&self) -> bool {
        self.spot_pairs.is_empty() && self.perp_pairs.is_empty()
    }

    pub fn add_spot_pairs(&mut self, pairs: Vec<String>) {
        self.spot_pairs.extend(pairs);
    }

    pub fn add_perp_pairs(&mut self, pairs: Vec<String>) {
        self.perp_pairs.extend(pairs);
    }

    pub fn remove_spot_pairs(&mut self, pairs: &[String]) {
        for pair in pairs {
            self.spot_pairs.remove(pair);
        }
    }

    pub fn remove_perp_pairs(&mut self, pairs: &[String]) {
        for pair in pairs {
            self.perp_pairs.remove(pair);
        }
    }

    /// Get the subscribed spot pairs.
    pub fn get_subscribed_spot_pairs(&self) -> Vec<String> {
        self.spot_pairs.iter().cloned().collect()
    }

    /// Get the subscribed perps pairs (without suffix).
    pub fn get_subscribed_perp_pairs(&self) -> Vec<String> {
        self.perp_pairs.iter().cloned().collect()
    }

    /// Get the subscribed perps pairs with the MARK suffix.
    pub fn get_fmt_subscribed_perp_pairs(&self) -> Vec<String> {
        self.perp_pairs
            .iter()
            .map(|pair| format!("{pair}:MARK"))
            .collect()
    }

    /// Get all the currently subscribed pairs.
    /// (Spot and Perp pairs with the suffix)
    pub fn get_fmt_subscribed_pairs(&self) -> Vec<String> {
        let mut spot_pairs = self.get_subscribed_spot_pairs();
        let perp_pairs = self.get_fmt_subscribed_perp_pairs();
        spot_pairs.extend(perp_pairs);
        spot_pairs
    }
}
