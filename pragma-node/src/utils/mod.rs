pub use custom_extractors::json_extractor::JsonExtractor;
pub use custom_extractors::path_extractor::PathExtractor;
pub use signing::typed_data::TypedData;

mod custom_extractors;
mod signing;

use bigdecimal::{BigDecimal};
use chrono::NaiveDateTime;

use crate::infra::repositories::entry_repository::MedianEntry;

/// Converts a currency pair to a pair id.
///
/// e.g "btc" and "usd" to "BTC/USD"
pub(crate) fn currency_pair_to_pair_id(quote: &str, base: &str) -> String {
    format!("{}/{}", quote.to_uppercase(), base.to_uppercase())
}

/// Computes the median price and time from a list of entries.
/// The median price is computed as the median of the median prices of each entry.
/// The median time is computed as the median of the times of each entry.
/// The median is computed as the middle value of a sorted list of values.
/// If the list has an even number of values, the median is computed as the average of the two middle values.
/// If the list is empty, None is returned.
pub(crate) fn compute_median_price_and_time(
    entries: &mut Vec<MedianEntry>,
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

    let latest_time = entries.last().unwrap().time;

    Some((median_price, latest_time))
}
