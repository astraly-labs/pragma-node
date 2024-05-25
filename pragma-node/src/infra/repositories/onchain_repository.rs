use bigdecimal::BigDecimal;
use diesel::sql_types::Numeric;
use diesel::{Queryable, QueryableByName, RunQueryDsl};

use pragma_entities::error::{adapt_infra_error, InfraError};
use pragma_monitoring::models::SpotEntry;

use crate::handlers::entries::{AggregationMode, Network, OnchainEntry};

#[derive(Queryable, QueryableByName)]
struct SpotEntryWithAggregatedPrice {
    #[diesel(embed)]
    pub spot_entry: SpotEntry,
    #[diesel(sql_type = Numeric)]
    pub aggregated_price: BigDecimal,
}

impl From<SpotEntryWithAggregatedPrice> for OnchainEntry {
    fn from(entry: SpotEntryWithAggregatedPrice) -> Self {
        OnchainEntry {
            publisher: entry.spot_entry.publisher,
            source: entry.spot_entry.source,
            price: entry.spot_entry.price.to_string(),
            tx_hash: entry.spot_entry.transaction_hash,
            timestamp: entry.spot_entry.timestamp.and_utc().timestamp() as u64,
        }
    }
}

// TODO(akhercha): Only works for Spot entries
pub async fn get_sources_and_aggregate(
    pool: &deadpool_diesel::postgres::Pool,
    network: Network,
    pair_id: String,
    timestamp: u64,
    aggregation_mode: AggregationMode,
) -> Result<(BigDecimal, Vec<OnchainEntry>), InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;

    // TODO(akhercha): put this somewhere else
    let table_name = match network {
        Network::Testnet => "spot_entry",
        Network::Mainnet => "mainnet_spot_entry",
    };

    // TODO(akhercha): Make this big SQL block more maintainable
    let aggregated_price_sql = match aggregation_mode {
        AggregationMode::Mean => "AVG(price) AS aggregated_price",
        AggregationMode::Median => {
            "(
            SELECT AVG(price)
            FROM (
                SELECT price
                FROM FilteredEntries
                ORDER BY price
                LIMIT 2 - (SELECT COUNT(*) FROM FilteredEntries) % 2
                OFFSET (SELECT (COUNT(*) - 1) / 2 FROM FilteredEntries)
            ) AS MedianPrices
        ) AS aggregated_price"
        }
        AggregationMode::Twap => Err(InfraError::InternalServerError)?,
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
            ),
            FilteredEntries AS (
                SELECT *
                FROM RankedEntries
                WHERE rn = 1
            ),
            AggregatedPrice AS (
                SELECT {aggregated_price_sql}
                FROM FilteredEntries
            )
            SELECT DISTINCT 
                FE.*,
                AP.aggregated_price
            FROM 
                FilteredEntries FE,
                AggregatedPrice AP
            ORDER BY 
                FE.timestamp DESC;
        "#,
        table_name,
        aggregated_price_sql = aggregated_price_sql
    );

    let raw_entries = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql)
                .bind::<diesel::sql_types::Text, _>(pair_id)
                .bind::<diesel::sql_types::BigInt, _>(timestamp as i64)
                .load::<SpotEntryWithAggregatedPrice>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    if raw_entries.is_empty() {
        return Ok((BigDecimal::from(0), vec![]));
    }

    let aggregated_price = raw_entries.first().unwrap().aggregated_price.clone();
    let entries: Vec<OnchainEntry> = raw_entries.into_iter().map(From::from).collect();
    Ok((aggregated_price, entries))
}

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
