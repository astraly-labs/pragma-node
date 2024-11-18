/// "PRAGMA" to number is bigger than 2**40 - we alias it to "PRGM" to fit in 40 bits.
/// Needed for StarkEx signature.
/// See:
/// https://docs.starkware.co/starkex/perpetual/becoming-an-oracle-provider-for-starkex.html
pub const PRAGMA_ORACLE_NAME_FOR_STARKEX: &str = "PRGM";

/// Used for the subscription to the entry websocket.
/// Represents the maximum interval from now that we check for entries.
/// If we don't have have any entries for that interval max, we stop searching.
pub const MAX_INTERVAL_WITHOUT_ENTRIES: u64 = 100000;

/// Used for the subscription to the entry websocket.
/// Represents the initial interval in milliseconds that we check for entries.
/// If there's no entries for that interval, we increase the interval by
/// INTERVAL_INCREMENT_IN_MS.
pub const INITAL_INTERVAL_IN_MS: u64 = 500;

/// Used for the subscription to the entry websocket.
/// Represents the increment in milliseconds that we increase the interval by.
/// If we reach MAX_INTERVAL_WITHOUT_ENTRIES, we stop searching.
pub const INTERVAL_INCREMENT_IN_MS: u64 = 500;

/// Used for the subscription to the entry websocket.
/// Represents the minimum number of unique publishers that we need to have
/// for a pair_id in order to return the computed price.
/// TODO: should be lower for development mode (1)
pub const MINIMUM_NUMBER_OF_PUBLISHERS: usize = 1;
