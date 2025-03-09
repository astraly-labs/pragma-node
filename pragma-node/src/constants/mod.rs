pub mod caches;
pub mod currencies;
pub mod others;
pub mod starkex_ws;

/// All offchain entries are quoted with 18 decimals.
///
/// This is not the case for on-chain entries! They still have indiviual decimals.
/// We use the `get_onchain_decimals` function to query the RPC and know how many.
pub const EIGHTEEN_DECIMALS: u32 = 18;
