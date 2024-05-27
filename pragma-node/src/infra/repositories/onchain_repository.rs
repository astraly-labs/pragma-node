use bigdecimal::BigDecimal;

use deadpool_diesel::postgres::Pool;
use diesel::sql_types::{BigInt, Numeric, Text};
use diesel::{Queryable, QueryableByName, RunQueryDsl};

use pragma_entities::error::{adapt_infra_error, InfraError};
use pragma_monitoring::models::SpotEntry;

use crate::handlers::entries::{AggregationMode, Network, OnchainEntry};

const BACKWARD_TIMESTAMP_INTERVAL: &str = "1 hour";

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
        _ => Err(InfraError::InternalServerError)?,
    };
    Ok(query)
}

fn get_table_name_from_network(network: Network) -> &'static str {
    match network {
        Network::Testnet => "spot_entry",
        Network::Mainnet => "mainnet_spot_entry",
    }
}

fn build_sql_query(network: Network, aggregation_mode: AggregationMode) -> String {
    format!(
        r#"
        WITH RankedEntries AS (
            SELECT 
                *,
                ROW_NUMBER() OVER (PARTITION BY publisher, source ORDER BY timestamp DESC) as rn
            FROM 
                {table_name}
            WHERE 
                pair_id = $1 
                AND timestamp BETWEEN (to_timestamp($2) - INTERVAL '{backward_interval}') AND to_timestamp($2)
        ),
        FilteredEntries AS (
            SELECT *
            FROM RankedEntries
            WHERE rn = 1
        ),
        AggregatedPrice AS (
            SELECT {aggregation_subquery}
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
        table_name = get_table_name_from_network(network),
        backward_interval = BACKWARD_TIMESTAMP_INTERVAL,
        aggregation_subquery = get_aggregation_query(aggregation_mode).unwrap()
    )
}

// TODO(akhercha): Only works for Spot entries
pub async fn get_sources_and_aggregate(
    pool: &Pool,
    network: Network,
    pair_id: String,
    timestamp: u64,
    aggregation_mode: AggregationMode,
) -> Result<(BigDecimal, Vec<OnchainEntry>), InfraError> {
    let raw_sql = build_sql_query(network, aggregation_mode);

    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let raw_entries = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql)
                .bind::<Text, _>(pair_id)
                .bind::<BigInt, _>(timestamp as i64)
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

// pub async fn get_last_updated_timestamp(
//     pool: &Pool,
//     network: Network,
//     pair_id: String,
// ) -> Result<u64, InfraError> {
//     let raw_sql = format!(
//         r#"
//         SELECT
//             *
//         FROM
//             {table_name}
//         WHERE
//             pair_id = $1
//         ORDER BY timestamp DESC
//         LIMIT 1;
//     "#,
//         table_name = get_table_name_from_network(network)
//     );

//     let conn = pool.get().await.map_err(adapt_infra_error)?;
//     let raw_entry = conn
//         .interact(move |conn| {
//             diesel::sql_query(raw_sql)
//                 .bind::<diesel::sql_types::Text, _>(pair_id)
//                 .load::<SpotEntry>(conn)
//         })
//         .await
//         .map_err(adapt_infra_error)?
//         .map_err(adapt_infra_error)?;

//     let most_recent_entry = raw_entry.first().ok_or(InfraError::NotFound)?;
//     Ok(most_recent_entry.timestamp.and_utc().timestamp() as u64)
// }
