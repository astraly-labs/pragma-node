pub mod starkex_ws;

/// "PRAGMA" to number is bigger than 2**40 - we alias it to "PRGM" to fit in 40 bits.
/// Needed for StarkEx signature.
/// See:
/// https://docs.starkware.co/starkex/perpetual/becoming-an-oracle-provider-for-starkex.html
pub const PRAGMA_ORACLE_NAME_FOR_STARKEX: &str = "PRGM";

/// We cache the update count for our onchain publishers because the query
/// takes a lot of time.
/// The cached value will live for the time specified in time to live.
/// See: https://docs.rs/moka/latest/moka/future/struct.Cache.html#example-time-based-expirations
pub const PUBLISHERS_UDPATES_CACHE_TIME_TO_LIVE_IN_SECONDS: u64 = 20 * 60; // 20 minutes
pub const PUBLISHERS_UDPATES_CACHE_TIME_TO_IDLE_IN_SECONDS: u64 = 5 * 60; // 20 minutes
