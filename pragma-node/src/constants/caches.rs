// See:
/// https://docs.rs/moka/latest/moka/future/struct.Cache.html#example-time-based-expirations

/// Cache the update count for our onchain publishers because the query
/// takes a lot of time.
/// The cached value will live for the time specified in time to live.
pub const PUBLISHERS_UDPATES_CACHE_TIME_TO_LIVE_IN_SECONDS: u64 = 20 * 60; // 20 minutes
pub const PUBLISHERS_UDPATES_CACHE_TIME_TO_IDLE_IN_SECONDS: u64 = 5 * 60; // 5 minutes

/// Cache of the stored Merkle Tree for a certain block in Redis.
/// Since this value never change we can cache it for faster iterations.
pub const MERKLE_FEED_TREE_CACHE_TIME_TO_LIVE_IN_SECONDS: u64 = 6 * 60; // 6 minutes
pub const MERKLE_FEED_TREE_CACHE_TIME_TO_IDLE_IN_SECONDS: u64 = 60; // 1 minutes
