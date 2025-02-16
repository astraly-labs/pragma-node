use bigdecimal::{num_bigint::BigInt, BigDecimal, Num};

/// Calculates the percentage difference between two hex-formatted prices
pub fn price_difference_percentage(price1: &str, price2: &str) -> BigDecimal {
    // Convert hex strings to BigInt first, removing "0x" prefix if present
    let price1 = price1.strip_prefix("0x").unwrap_or(price1);
    let price2 = price2.strip_prefix("0x").unwrap_or(price2);

    let price1_int = BigInt::from_str_radix(price1, 16).unwrap();
    let price2_int = BigInt::from_str_radix(price2, 16).unwrap();

    // Convert to BigDecimal for division
    let price1_dec = BigDecimal::from(price1_int);
    let price2_dec = BigDecimal::from(price2_int);

    // Calculate absolute difference
    let diff = if price1_dec > price2_dec {
        &price1_dec - &price2_dec
    } else {
        &price2_dec - &price1_dec
    };

    // Calculate percentage difference relative to the first price
    let hundred = BigDecimal::from(100);
    (&diff * &hundred) / &price1_dec
}

/// Checks if two hex-formatted prices are within the given threshold percentage
pub fn is_price_within_threshold(
    price1: &str,
    price2: &str,
    threshold_percentage: &BigDecimal,
) -> bool {
    let difference = price_difference_percentage(price1, price2);
    difference <= *threshold_percentage
}

/// Macro to assert that two prices are within a threshold
/// Prices must be provided as hex!
#[macro_export]
macro_rules! assert_hex_prices_within_threshold {
    ($price1:expr, $price2:expr, $threshold:expr) => {
        assert!(
            $crate::common::utils::is_price_within_threshold($price1, $price2, &$threshold),
            "Price difference exceeds {}%. Price 1: {}, Price 2: {}",
            $threshold,
            $price1,
            $price2
        );
    };
}
