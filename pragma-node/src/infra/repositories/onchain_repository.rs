use bigdecimal::BigDecimal;
use diesel::RunQueryDsl;

use pragma_entities::error::{adapt_infra_error, InfraError};
use pragma_monitoring::models::SpotEntry;

use crate::handlers::entries::{AggregationMode, Network, OnchainEntry};

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
    network: Network,
    pair_id: String,
    timestamp: u64,
) -> Result<Vec<OnchainEntry>, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;

    let table_name = match network {
        Network::Mainnet => "mainnet_spot_entry",
        Network::Testnet => "spot_entry",
    };

    let raw_sql = format!(
        r#"
            WITH RankedEntries AS (
                SELECT 
                    *,
                    ROW_NUMBER() OVER (PARTITION BY publisher, source ORDER BY timestamp DESC) as rn
                FROM 
                    {}
                WHERE 
                    pair_id = $1 
                    AND timestamp BETWEEN (to_timestamp($2) - INTERVAL '1 hour') AND to_timestamp($2)
            )
            SELECT 
                *
            FROM 
                RankedEntries
            WHERE 
                rn = 1
            ORDER BY 
                timestamp DESC;
        "#,
        table_name
    );

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

pub fn compute_price(
    components: &[OnchainEntry],
    aggregation_mode: AggregationMode,
) -> Result<String, InfraError> {
    let price = match aggregation_mode {
        AggregationMode::Median => compute_median_price(components),
        AggregationMode::Mean => compute_mean_price(components),
        AggregationMode::Twap => Err(InfraError::InternalServerError)?,
    };
    Ok(price)
}
fn compute_median_price(components: &[OnchainEntry]) -> String {
    let mut prices: Vec<BigDecimal> = components
        .iter()
        .map(|entry| entry.price.parse::<BigDecimal>().unwrap())
        .collect();
    prices.sort();

    let n = prices.len();
    let median = if n % 2 == 0 {
        let mid1 = &prices[n / 2];
        let mid2 = &prices[n / 2 - 1];
        (mid1 + mid2) / 2
    } else {
        prices[n / 2].clone()
    };
    median.to_string()
}

fn compute_mean_price(components: &[OnchainEntry]) -> String {
    let sum: BigDecimal = components
        .iter()
        .map(|entry| entry.price.parse::<BigDecimal>().unwrap())
        .sum();
    let n = BigDecimal::from(components.len() as u32);
    let mean = sum / n;
    mean.to_string()
}
