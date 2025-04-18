use std::collections::{HashMap, HashSet};

use pragma_common::{AggregationMode, InstrumentType, Interval, pair::Pair};
use pragma_entities::EntryError;
use serde::{Deserialize, Serialize};

use crate::{
    infra::repositories::entry_repository::{self, MedianEntry},
    state::AppState,
    utils::SubscriptionType,
};

use super::get_entry::EntryParams;

pub mod subscribe_to_entry;
pub mod subscribe_to_price;
pub use subscribe_to_entry::subscribe_to_entry;
pub use subscribe_to_price::subscribe_to_price;

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

pub async fn get_latest_entry(
    state: &AppState,
    pair: &Pair,
    is_routing: bool,
    entry_params: &EntryParams,
) -> Result<MedianEntry, EntryError> {
    // We have to update the timestamp to now every tick
    let mut new_routing = entry_params.clone();
    new_routing.timestamp = chrono::Utc::now().timestamp();

    let entry = entry_repository::routing(&state.offchain_pool, is_routing, pair, &new_routing)
        .await
        .map_err(EntryError::from)?;

    Ok(entry)
}

pub async fn get_latest_entries_multi_pair(
    state: &AppState,
    pairs: &[Pair],
    is_routing: bool,
    entry_params: &EntryParams,
) -> Result<HashMap<String, MedianEntry>, EntryError> {
    let mut latest_entries = HashMap::new();

    for pair in pairs {
        match get_latest_entry(state, pair, is_routing, entry_params).await {
            Ok(entry) => {
                // Add :MARK suffix to the key if it's a perp pair
                let key = if entry_params.data_type == InstrumentType::Perp {
                    format!("{}:MARK", pair.to_pair_id())
                } else {
                    pair.to_pair_id()
                };
                latest_entries.insert(key, entry);
            }
            Err(e) => {
                tracing::error!("❌ Failed to process message: {}", e);
            }
        }
    }

    // Return error only if we couldn't get any entries
    if latest_entries.is_empty() {
        return Err(EntryError::HistoryNotFound);
    }

    Ok(latest_entries)
}

pub fn get_params_for_websocket(is_perp: bool) -> EntryParams {
    EntryParams {
        interval: Interval::OneHundredMillisecond,
        timestamp: chrono::Utc::now().timestamp_millis(),
        aggregation_mode: AggregationMode::Median,
        data_type: if is_perp {
            InstrumentType::Perp
        } else {
            InstrumentType::Spot
        },
        expiry: String::default(),
        with_components: true,
    }
}
