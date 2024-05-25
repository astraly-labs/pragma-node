use bigdecimal::BigDecimal;
use diesel::RunQueryDsl;
use std::ops::Div;
use std::str::FromStr;

use pragma_entities::error::{adapt_infra_error, InfraError};
use pragma_monitoring::models::SpotEntry;

use crate::handlers::entries::AggregationMode;
use crate::handlers::entries::OnchainEntry;

pub async fn get_last_updated_timestamp(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
) -> Result<u64, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let raw_sql = r#"
        SELECT 
            *
        FROM 
            spot_entry
        ORDER BY timestamp DESC
        LIMIT 1;
    "#;

    let raw_entry = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql)
                .bind::<diesel::sql_types::Text, _>(pair_id)
                .load::<SpotEntry>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    let most_recent_entry = raw_entry.first().ok_or(InfraError::NotFound)?;

    Ok(most_recent_entry.timestamp.and_utc().timestamp() as u64)
}

pub async fn get_sources_for_pair(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
    timestamp: u64,
) -> Result<Vec<OnchainEntry>, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    // TODO(akhercha): Update back to interval of 1 hour after debugging
    let raw_sql = r#"
        WITH RankedEntries AS (
            SELECT 
                *,
                ROW_NUMBER() OVER (PARTITION BY publisher, source ORDER BY timestamp DESC) as rn
            FROM 
                spot_entry
            WHERE 
                pair_id = $1 
                AND timestamp BETWEEN (to_timestamp($2) - INTERVAL '30 minutes') AND to_timestamp($2)
        )
        SELECT 
            *
        FROM 
            RankedEntries
        WHERE 
            rn = 1
        ORDER BY 
            timestamp DESC;
    "#;

    let raw_entries = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql)
                .bind::<diesel::sql_types::Text, _>(pair_id)
                .bind::<diesel::sql_types::BigInt, _>(timestamp as i64)
                .load::<SpotEntry>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    if raw_entries.is_empty() {
        return Ok(vec![]);
    }

    // Adapt SpotEntry to OnchainEntry
    // TODO(akhercha): better way to handle this
    let entries: Vec<OnchainEntry> = raw_entries
        .iter()
        .map(|raw_entry: &SpotEntry| OnchainEntry {
            publisher: raw_entry.publisher.clone(),
            source: raw_entry.source.clone(),
            price: raw_entry.price.to_string(),
            tx_hash: raw_entry.transaction_hash.clone(),
            timestamp: raw_entry.timestamp.and_utc().timestamp() as u64,
        })
        .collect();
    Ok(entries)
}

pub fn compute_price(components: &[OnchainEntry], aggregation_mode: AggregationMode) -> String {
    match aggregation_mode {
        AggregationMode::Median => compute_median_price(components, 8),
        AggregationMode::Mean => compute_mean_price(components, 8),
        AggregationMode::Twap => panic!("Twap not implemented"),
    }
}

fn compute_median_price(components: &[OnchainEntry], decimals: u32) -> String {
    let mut prices: Vec<BigDecimal> = components
        .iter()
        .map(|entry| entry.price.parse::<BigDecimal>().unwrap())
        .collect();
    prices.sort();
    let n = prices.len();
    let median = if n % 2 == 0 {
        (prices[n / 2 - 1].clone() + prices[n / 2].clone()) / BigDecimal::from(2)
    } else {
        prices[n / 2].clone()
    };

    tracing::info!("Median: {:?}", median.to_string());

    let scale_factor = BigDecimal::from_str(&format!("1e{}", decimals)).unwrap();

    let scaled_median = median / scale_factor;

    let scaled_median_str = scaled_median.to_string();
    let integer_part = scaled_median_str.split('.').next().unwrap_or("0");

    integer_part.to_string()
}

fn compute_mean_price(components: &[OnchainEntry], decimals: u32) -> String {
    let sum: BigDecimal = components
        .iter()
        .map(|entry| entry.price.parse::<BigDecimal>().unwrap())
        .sum();
    let n = BigDecimal::from(components.len() as u32);

    let mean = sum / n;

    let scale_factor = BigDecimal::from_str(&format!("1e{}", decimals)).unwrap();

    let scaled_mean = mean.div(scale_factor);

    let scaled_mean_str = scaled_mean.to_string();
    let integer_part = scaled_mean_str.split('.').next().unwrap_or("0");

    integer_part.to_string()
}
