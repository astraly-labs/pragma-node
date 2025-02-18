pub mod aws;
pub mod conversion;
pub mod custom_extractors;
pub mod kafka;
pub mod macros;
pub mod pricer;
pub mod sql;
pub mod ws;

pub use aws::PragmaSignerBuilder;
pub use conversion::{convert_via_quote, format_bigdecimal_price, normalize_to_decimals};
pub use custom_extractors::path_extractor::PathExtractor;
pub use kafka::publish_to_kafka;
use moka::future::Cache;
use pragma_common::types::entries::Entry;
use pragma_common::types::pair::Pair;
use pragma_entities::dto::Publisher;
pub use ws::*;

use std::collections::HashMap;

use bigdecimal::num_bigint::ToBigInt;
use bigdecimal::{BigDecimal, ToPrimitive};
use chrono::NaiveDateTime;
use deadpool_diesel::postgres::Pool;
use pragma_common::types::Network;
use pragma_entities::{
    convert_timestamp_to_datetime, Entry as EntityEntry, EntryError, FutureEntry, NewEntry,
    PublisherError,
};
use starknet_crypto::{Felt, Signature};

use crate::infra::repositories::publisher_repository;
use crate::infra::repositories::{
    entry_repository::MedianEntry, onchain_repository::entry::get_existing_pairs,
};

const ONE_YEAR_IN_SECONDS: f64 = 3_153_600_f64;

/// From a map of currencies and their decimals, returns the number of decimals for a given pair.
/// If the currency is not found in the map, the default value is 8.
pub(crate) fn get_decimals_for_pair<S: ::std::hash::BuildHasher>(
    currencies: &HashMap<String, BigDecimal, S>,
    pair_id: &str,
) -> u32 {
    let pair = Pair::from(pair_id);
    let base_decimals = currencies
        .get(&pair.base)
        .map_or(8, |d| d.to_u32().unwrap_or(8));
    let quote_decimals = currencies
        .get(&pair.quote)
        .map_or(8, |d| d.to_u32().unwrap_or(8));
    std::cmp::min(base_decimals, quote_decimals)
}

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
pub(crate) async fn is_onchain_existing_pair(pool: &Pool, pair: &String, network: Network) -> bool {
    let existings_pairs = get_existing_pairs(pool, network)
        .await
        .expect("Couldn't get the existing pairs from the database.");

    existings_pairs.into_iter().any(|p| p.pair_id == *pair)
}

pub fn convert_entry_to_db(entry: &Entry, signature: &Signature) -> Result<NewEntry, EntryError> {
    let dt = convert_timestamp_to_datetime!(entry.base.timestamp)?;

    Ok(NewEntry {
        pair_id: entry.pair_id.clone(),
        publisher: entry.base.publisher.clone(),
        source: entry.base.source.clone(),
        timestamp: dt,
        publisher_signature: format!("0x{signature}"),
        price: entry.price.into(),
    })
}

/// Computes the volatility from a list of entries.
/// The volatility is computed as the annualized standard deviation of the log returns.
/// The log returns are computed as the natural logarithm of the ratio between two consecutive median prices.
/// The annualized standard deviation is computed as the square root of the variance multiplied by 10^8.
pub(crate) fn compute_volatility(entries: &[MedianEntry]) -> f64 {
    if entries.len() < 2 {
        return 0.0;
    }
    let mut values = Vec::new();
    for i in 1..entries.len() {
        if entries[i].median_price.to_f64().unwrap_or(0.0) > 0.0
            && entries[i - 1].median_price.to_f64().unwrap() > 0.0
            && (entries[i].time - entries[i - 1].time).num_seconds() > 0
        {
            let log_return = (entries[i].median_price.to_f64().unwrap()
                / entries[i - 1].median_price.to_f64().unwrap())
            .ln()
            .powi(2);

            let time = (entries[i].time - entries[i - 1].time)
                .num_seconds()
                .to_f64()
                .unwrap()
                / ONE_YEAR_IN_SECONDS;

            values.push((log_return, time));
        }
    }

    let variance: f64 = values.iter().map(|v| v.0 / v.1).sum::<f64>() / values.len() as f64;
    variance.sqrt() * 10_f64.powi(8)
}

/// Converts a big decimal price to a hex string 0x prefixed.
pub(crate) fn big_decimal_price_to_hex(price: &BigDecimal) -> String {
    format!(
        "0x{}",
        price.to_bigint().unwrap_or_default().to_str_radix(16)
    )
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
            .map_err(EntryError::InfraError)?
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn new_entry(median_price: u32, timestamp: i64) -> MedianEntry {
        MedianEntry {
            time: Utc
                .timestamp_opt(timestamp, 0)
                .single()
                .expect("Invalid timestamp")
                .naive_utc(),
            median_price: median_price.into(),
            num_sources: 5,
        }
    }

    #[test]
    fn test_compute_volatility_no_entries() {
        let entries = vec![];
        let epsilon = 1e-10;
        assert!((compute_volatility(&entries) - 0.0).abs() < epsilon);
    }

    #[test]
    fn test_compute_volatility_simple() {
        let entries = vec![new_entry(100, 1_640_995_200), new_entry(110, 1_641_081_600)];

        let expected_log_return = (110_f64 / 100_f64).ln().powi(2);
        let expected_time = f64::from(1_641_081_600 - 1_640_995_200) / ONE_YEAR_IN_SECONDS;
        let expected_variance = expected_log_return / expected_time;
        let expected_volatility = expected_variance.sqrt() * 10_f64.powi(8);
        let computed_volatility = compute_volatility(&entries);
        let epsilon: f64 = 1e-6;

        assert!((computed_volatility - expected_volatility).abs() < epsilon);
    }

    #[test]
    fn test_compute_volatility() {
        let entries = vec![
            new_entry(47_686, 1_640_995_200),
            new_entry(47_345, 1_641_081_600),
            new_entry(46_458, 1_641_168_000),
            new_entry(45_897, 1_641_254_400),
            new_entry(43_569, 1_641_340_800),
        ];

        let epsilon = 1e-10;
        assert!((compute_volatility(&entries) - 17_264_357.963_673_33).abs() < epsilon);
    }

    #[test]
    fn test_compute_volatility_zero_price() {
        let entries = vec![
            new_entry(47_686, 1_640_995_200),
            new_entry(0, 1_641_081_600),
            new_entry(46_458, 1_641_168_000),
        ];
        assert!(f64::is_nan(compute_volatility(&entries)));
    }

    #[test]
    fn test_compute_volatility_constant_prices() {
        let entries = vec![
            new_entry(47_686, 1_640_995_200),
            new_entry(47_686, 1_641_081_600),
            new_entry(47_686, 1_641_168_000),
            new_entry(47_686, 1_641_254_400),
            new_entry(47_686, 1_641_340_800),
        ];

        let epsilon = 1e-10;
        assert!((compute_volatility(&entries) - 0.0).abs() < epsilon);
    }

    #[test]
    fn test_compute_volatility_increasing_prices() {
        let entries = vec![
            new_entry(13_569, 1_640_995_200),
            new_entry(15_897, 1_641_081_600),
            new_entry(16_458, 1_641_168_000),
            new_entry(17_345, 1_641_254_400),
            new_entry(47_686, 1_641_340_800),
        ];

        let epsilon = 1e-10;
        assert!((compute_volatility(&entries) - 309_805_011.672_835_77).abs() < epsilon);
    }

    #[test]
    fn test_compute_volatility_decreasing_prices() {
        let entries = vec![
            new_entry(27_686, 1_640_995_200),
            new_entry(27_345, 1_641_081_600),
            new_entry(26_458, 1_641_168_000),
            new_entry(25_897, 1_641_254_400),
            new_entry(23_569, 1_641_340_800),
        ];

        let epsilon = 1e-10;
        assert!((compute_volatility(&entries) - 31_060_897.843_919_14_f64).abs() < epsilon);
    }
}
