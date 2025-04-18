use std::collections::HashMap;

use bigdecimal::BigDecimal;
use deadpool_diesel::postgres::Pool;
use diesel::sql_types::{BigInt, Integer, Numeric, Timestamp, VarChar};
use diesel::{Queryable, QueryableByName, RunQueryDsl};
use futures::future::try_join_all;
use moka::future::Cache;

use pragma_common::{InstrumentType, Pair, starknet::StarknetNetwork};
use pragma_entities::error::InfraError;

use crate::handlers::onchain::get_publishers::{Publisher, PublisherEntry};
use crate::infra::rpc::RpcClients;
use crate::utils::big_decimal_price_to_hex;

use super::{get_onchain_decimals, get_onchain_table_name};

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
    network: StarknetNetwork,
) -> Result<Vec<RawPublisher>, InfraError> {
    let address_column = match network {
        StarknetNetwork::Mainnet => "mainnet_address",
        StarknetNetwork::Sepolia => "testnet_address",
    };
    let raw_sql = format!(
        r"
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
    ",
    );

    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;
    let raw_publishers = conn
        .interact(move |conn| diesel::sql_query(raw_sql).load::<RawPublisher>(conn))
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

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
    pub async fn to_publisher_entry(
        &self,
        network: StarknetNetwork,
        decimals_cache: &Cache<StarknetNetwork, HashMap<String, u32>>,
        rpc_clients: &RpcClients,
    ) -> Result<PublisherEntry, InfraError> {
        let pair = Pair::from(self.pair_id.as_str());
        let decimals = get_onchain_decimals(decimals_cache, rpc_clients, network, &pair).await?;

        let entry = PublisherEntry {
            pair_id: self.pair_id.clone(),
            last_updated_timestamp: self.last_updated_timestamp.and_utc().timestamp() as u64,
            price: big_decimal_price_to_hex(&self.price),
            source: self.source.clone(),
            decimals,
            daily_updates: self.daily_updates as u32,
        };

        Ok(entry)
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
    publishers_updates_cache: &Cache<String, HashMap<String, RawPublisherUpdates>>,
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
        r"
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
        ",
    );

    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;
    let updates = conn
        .interact(move |conn| diesel::sql_query(raw_sql).load::<RawPublisherUpdates>(conn))
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

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
    network: StarknetNetwork,
    table_name: &str,
    publisher: &RawPublisher,
    publisher_updates: &RawPublisherUpdates,
    decimals_cache: &Cache<StarknetNetwork, HashMap<String, u32>>,
    rpc_clients: &RpcClients,
) -> Result<Publisher, InfraError> {
    let raw_sql_entries = format!(
        r"
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
    ",
        table_name = table_name,
        publisher_name = publisher.name
    );

    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;

    let raw_components = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql_entries).load::<RawLastPublisherEntryForPair>(conn)
        })
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    let component_futures: Vec<_> = raw_components
        .iter()
        .map(|component| component.to_publisher_entry(network, decimals_cache, rpc_clients))
        .collect();

    // Execute all futures concurrently and collect results
    let components = try_join_all(component_futures).await?;

    let last_updated_timestamp = components
        .iter()
        .map(|component| component.last_updated_timestamp)
        .max();

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

#[allow(clippy::implicit_hasher)]
pub async fn get_publishers_with_components(
    pool: &Pool,
    network: StarknetNetwork,
    data_type: InstrumentType,
    publishers: Vec<RawPublisher>,
    publishers_updates_cache: &Cache<String, HashMap<String, RawPublisherUpdates>>,
    decimals_cache: &Cache<StarknetNetwork, HashMap<String, u32>>,
    rpc_clients: &RpcClients,
) -> Result<Vec<Publisher>, InfraError> {
    let table_name = get_onchain_table_name(network, data_type)?;
    let publisher_names = publishers.iter().map(|p| p.name.clone()).collect();

    let updates =
        get_all_publishers_updates(pool, table_name, publisher_names, publishers_updates_cache)
            .await?;

    // Create a vector of futures for each publisher that needs processing
    let publisher_futures: Vec<_> = publishers
        .iter()
        .filter_map(|publisher| {
            // Only process publishers with updates
            let publisher_updates = updates.get(&publisher.name)?;
            if publisher_updates.daily_updates == 0 {
                return None;
            }

            let table_name = table_name.to_string();
            let publisher_updates = publisher_updates.clone();
            Some(async move {
                get_publisher_with_components(
                    pool,
                    network,
                    &table_name,
                    publisher,
                    &publisher_updates,
                    decimals_cache,
                    rpc_clients,
                )
                .await
            })
        })
        .collect();

    // Execute all publisher futures concurrently
    let publishers_response = try_join_all(publisher_futures).await?;

    Ok(publishers_response)
}
