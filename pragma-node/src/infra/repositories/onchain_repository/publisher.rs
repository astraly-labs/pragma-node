use std::collections::HashMap;

use bigdecimal::BigDecimal;
use deadpool_diesel::postgres::Pool;
use diesel::sql_types::{BigInt, Integer, Numeric, Timestamp, VarChar};
use diesel::{Queryable, QueryableByName, RunQueryDsl};

use moka::future::Cache;
use pragma_common::types::{DataType, Network};
use pragma_entities::error::{adapt_infra_error, InfraError};

use crate::handlers::onchain::get_publishers::{Publisher, PublisherEntry};
use crate::utils::{big_decimal_price_to_hex, get_decimals_for_pair};

use super::get_onchain_table_name;

#[derive(Debug, Queryable, QueryableByName)]
pub struct RawPublisher {
    #[diesel(sql_type = VarChar)]
    pub name: String,
    #[diesel(sql_type = VarChar)]
    pub website_url: String,
    #[diesel(sql_type = Integer)]
    pub publisher_type: i32,
}

pub async fn get_publishers(
    pool: &Pool,
    network: Network,
) -> Result<Vec<RawPublisher>, InfraError> {
    let address_column = match network {
        Network::Mainnet => "mainnet_address",
        Network::Sepolia => "testnet_address",
    };
    let raw_sql = format!(
        r#"
        SELECT
            name,
            website_url,
            publisher_type
        FROM
            publishers
        WHERE
            {address_column} IS NOT NULL
        ORDER BY
            name ASC;
    "#,
        address_column = address_column,
    );

    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let raw_publishers = conn
        .interact(move |conn| diesel::sql_query(raw_sql).load::<RawPublisher>(conn))
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    Ok(raw_publishers)
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
    #[diesel(sql_type = BigInt)]
    pub daily_updates: i64,
}

impl RawLastPublisherEntryForPair {
    pub fn to_publisher_entry(&self, currencies: &HashMap<String, BigDecimal>) -> PublisherEntry {
        PublisherEntry {
            pair_id: self.pair_id.clone(),
            last_updated_timestamp: self.last_updated_timestamp.and_utc().timestamp() as u64,
            price: big_decimal_price_to_hex(&self.price),
            source: self.source.clone(),
            decimals: get_decimals_for_pair(currencies, &self.pair_id),
            daily_updates: self.daily_updates as u32,
        }
    }
}

#[derive(Debug, Clone, Queryable, QueryableByName)]
pub struct RawPublisherUpdates {
    #[diesel(sql_type = VarChar)]
    pub publisher: String,
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
    publishers_updates_cache: Cache<String, HashMap<String, RawPublisherUpdates>>,
) -> Result<HashMap<String, RawPublisherUpdates>, InfraError> {
    let publishers_list = publishers_names.join("','");

    // Try to retrieve the latest available cached value, and return it if it exists
    let maybe_cached_value = publishers_updates_cache.get(&publishers_list).await;
    if let Some(cached_value) = maybe_cached_value {
        tracing::debug!("Found a cached value for publishers: {publishers_list} - using it.");
        return Ok(cached_value);
    }
    tracing::debug!("No cache found for publishers: {publishers_list}, fetching the database.");

    // ... else, fetch the value from the database
    let raw_sql = format!(
        r#"
        SELECT 
            publisher,
            COUNT(*) FILTER (WHERE timestamp >= NOW() - INTERVAL '1 day') AS daily_updates,
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

    let updates: HashMap<String, RawPublisherUpdates> = updates
        .into_iter()
        .map(|update| (update.publisher.clone(), update))
        .collect();

    // Update the cache with the latest value for the publishers
    publishers_updates_cache
        .insert(publishers_list.clone(), updates.clone())
        .await;

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
    WITH recent_entries AS (
        SELECT 
            pair_id,
            price,
            source,
            timestamp AS last_updated_timestamp
        FROM 
            {table_name}
        WHERE
            publisher = '{publisher_name}'
            AND timestamp >= NOW() - INTERVAL '1 day'
    ),
    ranked_entries AS (
        SELECT 
            pair_id,
            price,
            source,
            last_updated_timestamp,
            ROW_NUMBER() OVER (PARTITION BY pair_id, source ORDER BY last_updated_timestamp DESC) as rn,
            COUNT(*) OVER (PARTITION BY pair_id, source) as daily_updates
        FROM 
            recent_entries
    )
    SELECT 
        pair_id,
        price,
        source,
        last_updated_timestamp,
        daily_updates
    FROM 
        ranked_entries
    WHERE 
        rn = 1
    ORDER BY 
        pair_id, source ASC;
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
        .ok_or(InfraError::NotFound)?;

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
    network: Network,
    data_type: DataType,
    currencies: HashMap<String, BigDecimal>,
    publishers: Vec<RawPublisher>,
    publishers_updates_cache: Cache<String, HashMap<String, RawPublisherUpdates>>,
) -> Result<Vec<Publisher>, InfraError> {
    let table_name = get_onchain_table_name(&network, &data_type)?;
    let publisher_names = publishers.iter().map(|p| p.name.clone()).collect();

    let updates =
        get_all_publishers_updates(pool, table_name, publisher_names, publishers_updates_cache)
            .await?;
    let mut publishers_response = Vec::with_capacity(publishers.len());

    for publisher in publishers.iter() {
        let publisher_updates = match updates.get(&publisher.name) {
            Some(updates) => updates,
            None => continue,
        };
        if publisher_updates.daily_updates == 0 {
            continue;
        }
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
