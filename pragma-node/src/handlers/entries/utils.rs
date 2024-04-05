use bigdecimal::{BigDecimal, ToPrimitive};
use chrono::NaiveDateTime;

use crate::infra::repositories::entry_repository::MedianEntry;

/// Converts a currency pair to a pair id.
///
/// e.g "btc" and "usd" to "BTC/USD"
pub(crate) fn currency_pair_to_pair_id(base: &str, quote: &str) -> String {
    format!("{}/{}", base.to_uppercase(), quote.to_uppercase())
}

/// Computes the median price and time from a list of entries.
/// The median price is computed as the median of the median prices of each entry.
/// The median time is computed as the median of the times of each entry.
/// The median is computed as the middle value of a sorted list of values.
/// If the list has an even number of values, the median is computed as the average of the two middle values.
/// If the list is empty, None is returned.
#[allow(dead_code)]
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

/// Computes the volatility from a list of entries.
/// The volatility is computed as the annualized standard deviation of the log returns.
/// The log returns are computed as the natural logarithm of the ratio between two consecutive median prices.
/// The annualized standard deviation is computed as the square root of the variance multiplied by 10^8.
pub(crate) fn compute_volatility(entries: &Vec<MedianEntry>) -> f64 {
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
                / 3153600_f64; // One year in seconds

            values.push((log_return, time));
        }
    }

    let variance: f64 = values.iter().map(|v| v.0 / v.1).sum::<f64>() / values.len() as f64;

    variance.sqrt() * 10_f64.powi(8)
}

#[test]
fn test_volatility() {
    let entries = vec![
        MedianEntry {
            time: chrono::DateTime::from_timestamp(1640995200, 0)
                .unwrap()
                .naive_utc(),
            median_price: bigdecimal::BigDecimal::from(47686),
            num_sources: 5,
        },
        MedianEntry {
            time: chrono::DateTime::from_timestamp(1641081600, 0)
                .unwrap()
                .naive_utc(),
            median_price: bigdecimal::BigDecimal::from(47345),
            num_sources: 5,
        },
        MedianEntry {
            time: chrono::DateTime::from_timestamp(1641168000, 0)
                .unwrap()
                .naive_utc(),
            median_price: bigdecimal::BigDecimal::from(46458),
            num_sources: 5,
        },
        MedianEntry {
            time: chrono::DateTime::from_timestamp(1641254400, 0)
                .unwrap()
                .naive_utc(),
            median_price: bigdecimal::BigDecimal::from(45897),
            num_sources: 5,
        },
        MedianEntry {
            time: chrono::DateTime::from_timestamp(1641340800, 0)
                .unwrap()
                .naive_utc(),
            median_price: bigdecimal::BigDecimal::from(43569),
            num_sources: 5,
        },
    ];

    // TODO: add more tests
    // This value was computed using a python script
    assert_eq!(
        compute_volatility(&entries),
        54594693.50567423,
        "wrong volatility"
    );
}
