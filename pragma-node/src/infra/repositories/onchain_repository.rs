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

fn get_aggregation_query(aggregation_mode: AggregationMode) -> Result<&'static str, InfraError> {
    let query = match aggregation_mode {
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
    Ok(query)
}

fn get_table_name_from_network(network: Network) -> &'static str {
    match network {
        Network::Testnet => "spot_entry",
        Network::Mainnet => "mainnet_spot_entry",
    }
}

fn build_sql_query() -> String {
    r#"
        WITH RankedEntries AS (
            SELECT 
                *,
                ROW_NUMBER() OVER (PARTITION BY publisher, source ORDER BY timestamp DESC) as rn
            FROM 
                $1
            WHERE 
                pair_id = $2 
                AND timestamp BETWEEN (to_timestamp($3) - INTERVAL '1 hour') AND to_timestamp($3)
        ),
        FilteredEntries AS (
            SELECT *
            FROM RankedEntries
            WHERE rn = 1
        ),
        AggregatedPrice AS (
            SELECT $4
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
    "#
    .to_string()
}

// TODO(akhercha): Only works for Spot entries
pub async fn get_sources_and_aggregate(
    pool: &deadpool_diesel::postgres::Pool,
    network: Network,
    pair_id: String,
    timestamp: u64,
    aggregation_mode: AggregationMode,
) -> Result<(BigDecimal, Vec<OnchainEntry>), InfraError> {
    let table_name = get_table_name_from_network(network);
    let aggregation_query = get_aggregation_query(aggregation_mode)?;
    let raw_sql = build_sql_query();

    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let raw_entries = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql)
                .bind::<diesel::sql_types::Text, _>(table_name)
                .bind::<diesel::sql_types::Text, _>(pair_id)
                .bind::<diesel::sql_types::BigInt, _>(timestamp as i64)
                .bind::<diesel::sql_types::Text, _>(aggregation_query)
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
    let raw_sql = r#"
        SELECT 
            *
        FROM 
            spot_entry
        ORDER BY timestamp DESC
        LIMIT 1;
    "#;

    let conn = pool.get().await.map_err(adapt_infra_error)?;
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
