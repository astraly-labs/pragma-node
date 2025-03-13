// See:
// <https://docs.rs/moka/latest/moka/future/struct.Cache.html#example-time-based-expirations>

/// Cache the update count for our onchain publishers because the query
/// takes a lot of time.
/// The cached value will live for the time specified in time to live.
pub const PUBLISHERS_UDPATES_CACHE_TIME_TO_LIVE_IN_SECONDS: u64 = 20 * 60; // 20 minutes
pub const PUBLISHERS_UDPATES_CACHE_TIME_TO_IDLE_IN_SECONDS: u64 = 5 * 60; // 5 minutes

/// Cache of the onchain decimals.
///
/// In pragma-node, every decimals are 18 but this is not the case for onchain entries.
/// So we use this cache to fetch RPC results & store decimals for a specific `pair_id`
/// for a given network.
/// The values don't change often at all so we use 1 week.
pub const DECIMALS_TIME_TO_LIVE_IN_SECONDS: u64 = 604_800; // 1 week

/// Cache of the stored publishers in memory.
/// This cache is used to retrieve the `Publisher` object from the database
/// when creating new entries.
pub const PUBLISHERS_CACHE_TIME_TO_LIVE_IN_SECONDS: u64 = 30 * 60; // 30 minutes
pub const PUBLISHERS_CACHE_TIME_TO_IDLE_IN_SECONDS: u64 = 5 * 60; // 5 minutes
