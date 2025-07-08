use std::collections::HashMap;
use std::time::Duration;

use moka::future::Cache;

use pragma_common::starknet::StarknetNetwork;
use pragma_entities::dto::Publisher;

use crate::constants::caches::{
    DECIMALS_TIME_TO_LIVE_IN_SECONDS, PUBLISHERS_CACHE_TIME_TO_IDLE_IN_SECONDS,
    PUBLISHERS_CACHE_TIME_TO_LIVE_IN_SECONDS, PUBLISHERS_UDPATES_CACHE_TIME_TO_IDLE_IN_SECONDS,
    PUBLISHERS_UDPATES_CACHE_TIME_TO_LIVE_IN_SECONDS,
};
use crate::infra::repositories::onchain_repository::publisher::RawPublisherUpdates;

/// Structure responsible of holding our Databases caches.
/// All the caches are initialized empty with their associated time to live in the
/// constants module.
#[derive(Clone, Debug)]
pub struct CacheRegistry {
    onchain_publishers_updates: Cache<String, HashMap<String, RawPublisherUpdates>>,
    onchain_decimals: Cache<StarknetNetwork, HashMap<String, u32>>,
    publishers: Cache<String, Publisher>,
}

impl Default for CacheRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheRegistry {
    /// Initialize all of our caches empty.
    pub fn new() -> Self {
        let onchain_publishers_updates = Cache::builder()
            .time_to_live(Duration::from_secs(
                PUBLISHERS_UDPATES_CACHE_TIME_TO_LIVE_IN_SECONDS,
            )) // 30 minutes
            .time_to_idle(Duration::from_secs(
                PUBLISHERS_UDPATES_CACHE_TIME_TO_IDLE_IN_SECONDS,
            )) // 5 minutes
            .build();

        let onchain_decimals = Cache::builder()
            .time_to_live(Duration::from_secs(DECIMALS_TIME_TO_LIVE_IN_SECONDS))
            .build();

        let publishers = Cache::builder()
            .time_to_live(Duration::from_secs(
                PUBLISHERS_CACHE_TIME_TO_LIVE_IN_SECONDS,
            ))
            .time_to_idle(Duration::from_secs(
                PUBLISHERS_CACHE_TIME_TO_IDLE_IN_SECONDS,
            ))
            .build();

        Self {
            onchain_publishers_updates,
            onchain_decimals,
            publishers,
        }
    }

    pub const fn onchain_publishers_updates(
        &self,
    ) -> &Cache<String, HashMap<String, RawPublisherUpdates>> {
        &self.onchain_publishers_updates
    }

    pub const fn onchain_decimals(&self) -> &Cache<StarknetNetwork, HashMap<String, u32>> {
        &self.onchain_decimals
    }

    pub const fn publishers(&self) -> &Cache<String, Publisher> {
        &self.publishers
    }
}
