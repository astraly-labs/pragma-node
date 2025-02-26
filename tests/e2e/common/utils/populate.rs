use rand::Rng;
use std::ops::Range;

use crate::common::constants::{EMPTY_SIGNATURE, PAIRS, SOURCES, VARIATION_PERCENTAGE};

pub fn get_pair_price(pair: &str) -> u128 {
    let price = match pair {
        "BTC/USD" => 88000.0, // 88 000$
        "ETH/USD" => 2700.0, // 2700$
        "SOL/USD" => 130.0, // 130$
        "STRK/USD" => 0.2, // 0,2$
        "DOGE/USD" => 0.2, // 0,2$
        _ => panic!("Pair not found, add it to the const PAIRS"),
    };

    (price * 10.0_f64.powi(8)) as u128
}

pub fn generate_entries(num_entries: u32, timestamp: u64) -> Vec<String> {
    let mut rng = rand::rng();
    let mut entries = Vec::with_capacity(num_entries as usize);

    // Generate at least one entry for each pair
    for &pair in PAIRS {
        entries.push(generate_entry(pair, SOURCES[0], timestamp));
    }

    // Generate remaining random entries
    for i in PAIRS.len()..num_entries as usize {
        let pair = PAIRS[rng.random_range(0..PAIRS.len())];
        let source = SOURCES[i % SOURCES.len()];
        // Alterate timestamp to note have duplicate entries
        entries.push(generate_entry(pair, source, timestamp - 1 - i as u64));
    }
    
    entries
}

fn price_range(price: u128, percentage: f64) -> Range<u128> {
    let delta = (price as f64 * (percentage / 100.0)).round() as u128;
    let lower_bound = price.saturating_sub(delta);
    let upper_bound = price.saturating_add(delta);

    lower_bound..upper_bound
}

pub fn generate_entry(pair: &str, source: &str, timestamp: u64) -> String {
    let mut rng = rand::rng();
    
    let price = get_pair_price(pair);
    let random_price: u128 = rng.random_range(price_range(price, VARIATION_PERCENTAGE));

    entry_from(pair, timestamp, random_price, source)
}

pub fn entry_from(pair: &str, timestamp: u64, price: u128, source: &str) -> String {
    format!(
        r#"
        INSERT INTO entries (
            pair_id,
            publisher,
            timestamp,
            price,
            source,
            publisher_signature
        ) VALUES (
            '{pair}',
            'TEST_PUBLISHER',
            to_timestamp({timestamp}),
            {price},
            '{source}',
            '{EMPTY_SIGNATURE}'
        );
    "#
    )
}

