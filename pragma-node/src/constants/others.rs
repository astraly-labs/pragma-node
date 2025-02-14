/// Used to determine the freshness of the routing.
///
/// For example, if we want to get the price of ETH/BTC,
/// we check if we have the pair ETH/BTC already being published AND
/// if the last time we got the price for ETH/BTC is less than
/// `ROUTING_FRESHNESS_THRESHOLD` seconds ago.
/// Otherwise, we return the price by routing through USD pairs.
pub const ROUTING_FRESHNESS_THRESHOLD: i64 = 60; // 1 minute
