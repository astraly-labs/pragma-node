pub mod conversion;
pub mod custom_extractors;
pub mod macros;
pub mod signing;
pub mod sql;
pub mod starkex;
pub mod ws;

pub use conversion::{convert_via_quote, format_bigdecimal_price, normalize_to_decimals};
pub use custom_extractors::path_extractor::PathExtractor;
use pragma_common::starknet::StarknetNetwork;
pub use ws::*;

use bigdecimal::BigDecimal;
use bigdecimal::num_bigint::{BigUint, ToBigInt};
use chrono::NaiveDateTime;
use deadpool_diesel::postgres::Pool;
use moka::future::Cache;
use pragma_entities::dto::Publisher;
use pragma_entities::{Entry as EntityEntry, EntryError, FutureEntry, PublisherError};
use starknet_crypto::Felt;

use crate::infra::repositories::publisher_repository;
use crate::infra::repositories::{
    entry_repository::MedianEntry, onchain_repository::entry::get_existing_pairs,
};

/// Returns the mid price between two prices.
pub fn get_mid_price(low: &BigDecimal, high: &BigDecimal) -> BigDecimal {
    (low + high) / BigDecimal::from(2)
}

/// Computes the median price and time from a list of entries.
/// The median price is computed as the median of the median prices of each entry.
/// The median time is computed as the median of the times of each entry.
/// The median is computed as the middle value of a sorted list of values.
/// If the list has an even number of values, the median is computed as the average of the two middle values.
/// If the list is empty, None is returned.
#[allow(dead_code)]
pub(crate) fn compute_median_price_and_time(
    entries: &mut [MedianEntry],
) -> Option<(BigDecimal, NaiveDateTime)> {
    if entries.is_empty() {
        return None;
    }

    // Compute median price
    entries.sort_by(|a, b| a.median_price.cmp(&b.median_price));
    let mid = entries.len() / 2;
    let median_price = if entries.len() % 2 == 0 {
        (&entries[mid - 1].median_price + &entries[mid].median_price) / BigDecimal::from(2)
    } else {
        entries[mid].median_price.clone()
    };

    entries.last().map(|entry| (median_price, entry.time))
}

/// Given a pair and a network, returns if it exists in the
/// onchain database.
pub(crate) async fn is_onchain_existing_pair(
    pool: &Pool,
    pair: &String,
    network: StarknetNetwork,
) -> bool {
    let existings_pairs = get_existing_pairs(pool, network)
        .await
        .expect("Couldn't get the existing pairs from the database.");

    existings_pairs.into_iter().any(|p| p.pair_id == *pair)
}

// Given a pair_id, check if it exists on the offchain db
pub(crate) async fn is_existing_pair(pool: &Pool, pair_id: &String) -> bool {
    let pair_id_owned = pair_id.clone();
    let conn = pool.get().await.expect("Couldn't connect to the database.");
    conn.interact(move |conn| EntityEntry::exists(conn, pair_id_owned))
        .await
        .expect("Couldn't check if pair exists")
        .expect("Couldn't get table result")
}

/// Converts a big decimal price to a hex string 0x prefixed.
pub(crate) fn big_decimal_price_to_hex(price: &BigDecimal) -> String {
    format!(
        "0x{}",
        price.to_bigint().unwrap_or_default().to_str_radix(16)
    )
}

pub(crate) fn hex_string_to_bigdecimal(
    hex_str: &str,
) -> Result<BigDecimal, Box<dyn std::error::Error>> {
    // Remove "0x" prefix if present
    let cleaned_hex = hex_str.trim_start_matches("0x");

    // Parse hex string to BigUint
    let parsed_big_int =
        BigUint::parse_bytes(cleaned_hex.as_bytes(), 16).ok_or("Failed to parse hex string")?;
    let big_int = parsed_big_int
        .to_bigint()
        .ok_or("Failed to convert to BigInt")?;
    let decimal = BigDecimal::new(big_int, 0);

    Ok(decimal)
}
/// Given a list of pairs, only return the ones that exists in the
/// database in separate lists.
/// TODO: handle future pairs?
/// A list of pairs can contains:
/// - Spot pairs: formatted as usual (e.g. "BTC/USD")
/// - Perpetual pairs: usual pair with a mark suffix (e.g. "BTC/USD:MARK").
pub(crate) async fn only_existing_pairs(
    pool: &Pool,
    pairs: Vec<String>,
) -> (
    Vec<String>, // spot pairs
    Vec<String>, // perpetual pairs
                 // TODO: future_pairs
) {
    let conn = pool.get().await.expect("Couldn't connect to the database.");

    let pairs = pairs
        .iter()
        .map(|pair| pair.to_uppercase().trim().to_string())
        .collect::<Vec<String>>();

    // Check spot entries
    let spot_pairs = pairs
        .iter()
        .filter(|pair| !pair.contains(':'))
        .map(ToString::to_string)
        .collect::<Vec<String>>();
    let spot_pairs = conn
        .interact(move |conn| EntityEntry::get_existing_pairs(conn, spot_pairs))
        .await
        .expect("Couldn't check if pair exists")
        .expect("Couldn't get table result");

    // Check perp entries
    let perp_pairs = pairs
        .iter()
        .filter(|pair| pair.contains(":MARK"))
        .map(|pair| pair.replace(":MARK", ""))
        .collect::<Vec<String>>();

    let perp_pairs = conn
        .interact(move |conn| FutureEntry::get_existing_perp_pairs(conn, perp_pairs))
        .await
        .expect("Couldn't check if pair exists")
        .expect("Couldn't get table result")
        .into_iter()
        .collect::<Vec<String>>();

    (spot_pairs, perp_pairs)
}

/// Validate publisher and return public key and account address
/// i.e check if the publisher is active and if the account address is correct
///
///
/// Arguments:
/// * `pool`: Database pool
/// * `publisher_name`: Publisher name
///
/// Returns:
/// * `(public_key, account_address)`: Public key and account address
pub async fn validate_publisher(
    pool: &Pool,
    publisher_name: &str,
    publishers_cache: &Cache<String, Publisher>,
) -> Result<(Felt, Felt), EntryError> {
    let publisher = if let Some(cached_value) = publishers_cache.get(publisher_name).await {
        tracing::debug!("Found a cached value for publisher: {publisher_name} - using it.");
        cached_value
    } else {
        tracing::debug!("No cache found for publisher: {publisher_name}, fetching the database.");
        publisher_repository::get(pool, publisher_name.to_string())
            .await
            .map_err(|_| EntryError::PublisherNotFound(publisher_name.into()))?
    };

    publisher.assert_is_active()?;

    let public_key = Felt::from_hex(&publisher.active_key).map_err(|_| {
        EntryError::PublisherError(PublisherError::InvalidKey(publisher.active_key))
    })?;

    let account_address = Felt::from_hex(&publisher.account_address).map_err(|_| {
        EntryError::PublisherError(PublisherError::InvalidAddress(publisher.account_address))
    })?;

    Ok((public_key, account_address))
}
