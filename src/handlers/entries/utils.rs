use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;

use crate::infra::repositories::entry_repository::MedianEntry;

/// Converts a currency pair to a pair id.
pub(crate) fn currency_pair_to_pair_id(quote: &str, base: &str) -> String {
    format!("{}/{}", quote.to_uppercase(), base.to_uppercase())
}

/// Computes the median price and time from a list of entries.
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

    // Compute median time
    entries.sort_by(|a, b| a.time.cmp(&b.time));
    let median_time = if entries.len() % 2 == 0 {
        entries[mid - 1].time
    } else {
        entries[mid].time
    };

    Some((median_price, median_time))
}
