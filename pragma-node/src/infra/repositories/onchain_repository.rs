use std::collections::HashMap;

use bigdecimal::{BigDecimal, ToPrimitive};
use deadpool_diesel::postgres::Pool;
use diesel::sql_types::{BigInt, Integer, Numeric, Text, Timestamp, VarChar};
use diesel::{Queryable, QueryableByName, RunQueryDsl};

use pragma_common::types::{AggregationMode, Network};
use pragma_entities::error::{adapt_infra_error, InfraError};
use pragma_monitoring::models::SpotEntry;

use crate::handlers::entries::utils::pair_id_to_currency_pair;
use crate::handlers::entries::{Checkpoint, OnchainEntry, Publisher, PublisherEntry};
use crate::utils::format_bigdecimal_price;

const BACKWARD_TIMESTAMP_INTERVAL: &str = "1 hour";

#[allow(dead_code)]
enum DataType {
    SpotEntry,
    SpotCheckpoint,
    FutureEntry,
}

fn get_table_name(network: Network, data_type: DataType) -> &'static str {
    match (network, data_type) {
        (Network::Testnet, DataType::SpotEntry) => "spot_entry",
        (Network::Mainnet, DataType::SpotEntry) => "mainnet_spot_entry",
        (Network::Testnet, DataType::SpotCheckpoint) => "spot_checkpoints",
        (Network::Mainnet, DataType::SpotCheckpoint) => "mainnet_spot_checkpoints",
        // TODO(akhercha): Future tables not used yet
        (Network::Testnet, DataType::FutureEntry) => "future_entry",
        (Network::Mainnet, DataType::FutureEntry) => "mainnet_future_entry",
    }
}

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

fn build_sql_query(
    network: Network,
    aggregation_mode: AggregationMode,
) -> Result<String, InfraError> {
    let aggregation_query = get_aggregation_query(aggregation_mode)?;
    let complete_sql_query = format!(
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
        table_name = get_table_name(network, DataType::SpotEntry),
        backward_interval = BACKWARD_TIMESTAMP_INTERVAL,
        aggregation_subquery = aggregation_query
    );
    Ok(complete_sql_query)
}

// TODO(akhercha): Only works for Spot entries
pub async fn get_sources_and_aggregate(
    pool: &Pool,
    network: Network,
    pair_id: String,
    timestamp: u64,
    aggregation_mode: AggregationMode,
) -> Result<(BigDecimal, Vec<OnchainEntry>), InfraError> {
    let raw_sql = build_sql_query(network, aggregation_mode)?;

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

#[derive(Queryable, QueryableByName)]
struct EntryTimestamp {
    #[diesel(sql_type = Timestamp)]
    pub timestamp: chrono::NaiveDateTime,
}

// TODO(akhercha): Only works for Spot entries
// TODO(akhercha): Give different result than onchain oracle sometimes
pub async fn get_last_updated_timestamp(
    pool: &Pool,
    network: Network,
    pair_id: String,
) -> Result<u64, InfraError> {
    let raw_sql = format!(
        r#"
        SELECT
            timestamp
        FROM
            {table_name}
        WHERE
            pair_id = $1
        ORDER BY timestamp DESC
        LIMIT 1;
    "#,
        table_name = get_table_name(network, DataType::SpotEntry)
    );

    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let raw_entry = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql)
                .bind::<diesel::sql_types::Text, _>(pair_id)
                .load::<EntryTimestamp>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    let most_recent_entry = raw_entry.first().ok_or(InfraError::NotFound)?;
    Ok(most_recent_entry.timestamp.and_utc().timestamp() as u64)
}

#[derive(Queryable, QueryableByName)]
struct RawCheckpoint {
    #[diesel(sql_type = VarChar)]
    pub transaction_hash: String,
    #[diesel(sql_type = Numeric)]
    pub price: BigDecimal,
    #[diesel(sql_type = Timestamp)]
    pub timestamp: chrono::NaiveDateTime,
    #[diesel(sql_type = VarChar)]
    pub sender_address: String,
}

impl RawCheckpoint {
    pub fn to_checkpoint(&self, decimals: u32) -> Checkpoint {
        Checkpoint {
            tx_hash: self.transaction_hash.clone(),
            price: format_bigdecimal_price(self.price.clone(), decimals),
            timestamp: self.timestamp.and_utc().timestamp() as u64,
            sender_address: self.sender_address.clone(),
        }
    }
}

pub async fn get_checkpoints(
    pool: &Pool,
    network: Network,
    pair_id: String,
    decimals: u32,
    limit: u64,
) -> Result<Vec<Checkpoint>, InfraError> {
    let raw_sql = format!(
        r#"
        SELECT
            transaction_hash,
            price,
            timestamp,
            sender_address
        FROM
            {table_name}
        WHERE
            pair_id = $1
        ORDER BY timestamp DESC
        LIMIT $2;
    "#,
        table_name = get_table_name(network, DataType::SpotCheckpoint)
    );

    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let raw_checkpoints = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql)
                .bind::<diesel::sql_types::Text, _>(pair_id)
                .bind::<diesel::sql_types::BigInt, _>(limit as i64)
                .load::<RawCheckpoint>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    let checkpoints: Vec<Checkpoint> = raw_checkpoints
        .into_iter()
        .map(|raw_checkpoint| raw_checkpoint.to_checkpoint(decimals))
        .collect();
    Ok(checkpoints)
}

#[derive(Debug, Queryable, QueryableByName)]
pub struct RawPublisher {
    #[diesel(sql_type = VarChar)]
    pub name: String,
    #[diesel(sql_type = VarChar)]
    pub website_url: String,
    #[diesel(sql_type = Integer)]
    pub publisher_type: i32,
}

pub async fn get_publishers(pool: &Pool) -> Result<Vec<RawPublisher>, InfraError> {
    let raw_sql = r#"
        SELECT DISTINCT
            name,
            website_url,
            publisher_type
        FROM
            publishers
        ORDER BY
            name ASC;
    "#;

    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let raw_publishers = conn
        .interact(move |conn| diesel::sql_query(raw_sql).load::<RawPublisher>(conn))
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    Ok(raw_publishers)
}

fn get_decimals_for_pair(pair_id: &str, currencies: &HashMap<String, BigDecimal>) -> u32 {
    let (base, quote) = pair_id_to_currency_pair(pair_id);
    let base_decimals = match currencies.get(&base) {
        Some(decimals) => decimals.to_u32().unwrap_or_default(),
        None => 8,
    };
    let quote_decimals = match currencies.get(&quote) {
        Some(decimals) => decimals.to_u32().unwrap_or_default(),
        None => 8,
    };
    std::cmp::min(base_decimals, quote_decimals)
}

#[derive(Debug, Queryable, QueryableByName)]
pub struct RawLastPublisherEntryForPair {
    #[diesel(sql_type = VarChar)]
    pub pair_id: String,
    #[diesel(sql_type = Numeric)]
    pub price: BigDecimal,
    #[diesel(sql_type = VarChar)]
    pub source: String,
    #[diesel(sql_type = Timestamp)]
    pub last_updated_timestamp: chrono::NaiveDateTime,
}

impl RawLastPublisherEntryForPair {
    pub fn to_publisher_entry(&self, currencies: &HashMap<String, BigDecimal>) -> PublisherEntry {
        let decimals = get_decimals_for_pair(&self.pair_id, currencies);
        PublisherEntry {
            pair_id: self.pair_id.clone(),
            last_updated_timestamp: self.last_updated_timestamp.and_utc().timestamp() as u64,
            price: format_bigdecimal_price(self.price.clone(), decimals),
            source: self.source.clone(),
            decimals,
        }
    }
}

#[derive(Debug, Queryable, QueryableByName)]
pub struct RawPublisherUpdates {
    #[diesel(sql_type = BigInt)]
    pub daily_updates: i64,
    #[diesel(sql_type = BigInt)]
    pub total_updates: i64,
    #[diesel(sql_type = BigInt)]
    pub nb_feeds: i64,
}

async fn get_all_publishers_updates(
    pool: &Pool,
    table_name: &str,
    publishers_names: Vec<String>,
) -> Result<Vec<RawPublisherUpdates>, InfraError> {
    let publishers_list = publishers_names.join("','");
    let raw_sql = format!(
        r#"
        SELECT 
            publisher,
            COUNT(*) FILTER (WHERE block_timestamp >= NOW() - INTERVAL '1 day') AS daily_updates,
            COUNT(*) AS total_updates,
            COUNT(DISTINCT pair_id) AS nb_feeds
        FROM 
            {table_name}
        WHERE 
            publisher IN ('{publishers_list}')
        GROUP BY 
            publisher;
        "#,
        table_name = table_name,
        publishers_list = publishers_list,
    );

    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let updates = conn
        .interact(move |conn| diesel::sql_query(raw_sql).load::<RawPublisherUpdates>(conn))
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    Ok(updates)
}

async fn get_publisher_with_components(
    pool: &Pool,
    table_name: &str,
    publisher: &RawPublisher,
    publisher_updates: &RawPublisherUpdates,
    currencies: &HashMap<String, BigDecimal>,
) -> Result<Publisher, InfraError> {
    let raw_sql_entries = format!(
        r#"
        SELECT
            entries.pair_id,
            entries.price,
            entries.source,
            entries.timestamp as last_updated_timestamp
        FROM
            {table_name} entries
        INNER JOIN (
            SELECT
                pair_id,
                MAX(timestamp) AS max_timestamp
            FROM
                {table_name}
            WHERE
                publisher = '{publisher_name}'
            GROUP BY
                pair_id
        ) AS latest ON entries.pair_id = latest.pair_id AND entries.timestamp = latest.max_timestamp
        WHERE
            entries.publisher = '{publisher_name}'
        ORDER BY
            entries.pair_id, entries.source ASC;
        "#,
        table_name = table_name,
        publisher_name = publisher.name
    );

    let conn = pool.get().await.map_err(adapt_infra_error)?;

    let raw_components = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql_entries).load::<RawLastPublisherEntryForPair>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    let components: Vec<PublisherEntry> = raw_components
        .into_iter()
        .map(|component| component.to_publisher_entry(currencies))
        .collect();

    let last_updated_timestamp = components
        .iter()
        .map(|component| component.last_updated_timestamp)
        .max()
        .unwrap_or_default();

    let publisher = Publisher {
        publisher: publisher.name.clone(),
        website_url: publisher.website_url.clone(),
        last_updated_timestamp,
        r#type: publisher.publisher_type as u32,
        nb_feeds: publisher_updates.nb_feeds as u32,
        daily_updates: publisher_updates.daily_updates as u32,
        total_updates: publisher_updates.total_updates as u32,
        components,
    };
    Ok(publisher)
}

pub async fn get_publishers_with_components(
    pool: &Pool,
    publishers: Vec<RawPublisher>,
    network: Network,
    currencies: HashMap<String, BigDecimal>,
) -> Result<Vec<Publisher>, InfraError> {
    let table_name = get_table_name(network, DataType::SpotEntry);
    let publisher_names = publishers.iter().map(|p| p.name.clone()).collect();

    let updates = get_all_publishers_updates(pool, table_name, publisher_names).await?;

    let mut publishers_response = Vec::with_capacity(publishers.len());
    for (publisher, publisher_updates) in publishers.iter().zip(updates.iter()) {
        let publisher_with_components = get_publisher_with_components(
            pool,
            table_name,
            publisher,
            publisher_updates,
            &currencies,
        )
        .await?;
        publishers_response.push(publisher_with_components);
    }

    Ok(publishers_response)
}
