use std::collections::HashMap;

use bigdecimal::BigDecimal;
use deadpool_diesel::postgres::Pool;
use diesel::sql_types::{BigInt, Integer, Numeric, Timestamp, VarChar};
use diesel::{Queryable, QueryableByName, RunQueryDsl};
use futures::future::try_join_all;
use moka::future::Cache;

use pragma_common::{Pair, starknet::StarknetNetwork};
use pragma_entities::error::InfraError;

use crate::caches::CacheRegistry;
use crate::handlers::onchain::get_publishers::{
    GetOnchainPublishersParams, Publisher, PublisherEntry,
};
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

#[derive(Debug, Clone, Queryable, QueryableByName)]
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
        let pair = Pair::try_from(self.pair_id.as_str())
            .map_err(|e| InfraError::PairNotFound(e.to_string()))?;
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
    let cache_key = format!("{table_name}:{publishers_list}");

    // Try to retrieve the latest available cached value, and return it if it exists
    let maybe_cached_value = publishers_updates_cache.get(&cache_key).await;
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
        .insert(cache_key, updates.clone())
        .await;

    Ok(updates)
}

async fn get_publisher_with_components(
    pool: &Pool,
    params: &GetOnchainPublishersParams,
    table_name: &str,
    publisher: &RawPublisher,
    publisher_updates: &RawPublisherUpdates,
    caches: &CacheRegistry,
    rpc_clients: &RpcClients,
) -> Result<Publisher, InfraError> {
    let include_history = params.publisher.is_some();
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
            publisher = $1
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
    );
    let publisher_name = publisher.name.clone();

    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;

    let recent_components = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql_entries)
                .bind::<diesel::sql_types::Text, _>(publisher_name)
                .load::<RawLastPublisherEntryForPair>(conn)
        })
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    // The historical cache loader checks out another connection from the same pool.
    drop(conn);

    let raw_components = if include_history {
        let snapshot = get_publisher_history(
            pool,
            table_name,
            &publisher.name,
            caches.onchain_publisher_history(),
        )
        .await?;
        merge_history(snapshot, recent_components)
    } else {
        recent_components
    };

    let component_futures: Vec<_> = raw_components
        .iter()
        .map(|component| {
            component.to_publisher_entry(params.network, caches.onchain_decimals(), rpc_clients)
        })
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
        nb_feeds: if include_history {
            components
                .iter()
                .map(|entry| &entry.pair_id)
                .collect::<std::collections::HashSet<_>>()
                .len() as u32
        } else {
            publisher_updates.nb_feeds as u32
        },
        daily_updates: if include_history {
            components.iter().map(|entry| entry.daily_updates).sum()
        } else {
            publisher_updates.daily_updates as u32
        },
        total_updates: publisher_updates.total_updates as u32,
        components,
    };
    Ok(publisher)
}

#[allow(clippy::implicit_hasher)]
pub async fn get_publishers_with_components(
    pool: &Pool,
    params: &GetOnchainPublishersParams,
    publishers: Vec<RawPublisher>,
    caches: &CacheRegistry,
    rpc_clients: &RpcClients,
) -> Result<Vec<Publisher>, InfraError> {
    let include_history = params.publisher.is_some();
    let table_name = get_onchain_table_name(params.network, params.data_type);
    let publisher_names = publishers.iter().map(|p| p.name.clone()).collect();

    let updates = get_all_publishers_updates(
        pool,
        table_name,
        publisher_names,
        caches.onchain_publishers_updates(),
    )
    .await?;

    // Create a vector of futures for each publisher that needs processing
    let publisher_futures: Vec<_> = publishers
        .iter()
        .filter_map(|publisher| {
            // Only process publishers with updates
            let publisher_updates = updates.get(&publisher.name)?;
            if publisher_updates.daily_updates == 0 && !include_history {
                return None;
            }

            let table_name = table_name.to_string();
            let publisher_updates = publisher_updates.clone();
            Some(async move {
                get_publisher_with_components(
                    pool,
                    params,
                    &table_name,
                    publisher,
                    &publisher_updates,
                    caches,
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

fn merge_history(
    historical: Vec<RawLastPublisherEntryForPair>,
    recent: Vec<RawLastPublisherEntryForPair>,
) -> Vec<RawLastPublisherEntryForPair> {
    let mut sources = std::collections::BTreeMap::new();
    for mut entry in historical {
        entry.daily_updates = 0;
        sources.insert((entry.pair_id.clone(), entry.source.clone()), entry);
    }
    for entry in recent {
        sources.insert((entry.pair_id.clone(), entry.source.clone()), entry);
    }
    sources.into_values().collect()
}

async fn get_publisher_history(
    pool: &Pool,
    table_name: &str,
    publisher_name: &str,
    history_cache: &Cache<String, Vec<RawLastPublisherEntryForPair>>,
) -> Result<Vec<RawLastPublisherEntryForPair>, InfraError> {
    let key = format!("{table_name}:{publisher_name}");
    history_cache
        .try_get_with(key, async {
            let conn = pool.get().await.map_err(|error| error.to_string())?;
            let name = publisher_name.to_owned();
            let sql = format!(
                r"
                SELECT DISTINCT ON (pair_id, source) pair_id, source, price,
                    timestamp AT TIME ZONE 'UTC' AS last_updated_timestamp,
                    0::bigint AS daily_updates
                FROM {table_name} WHERE publisher = $1
                ORDER BY pair_id, source, timestamp DESC, transaction_hash DESC
            "
            );
            conn.interact(move |conn| {
                diesel::sql_query(sql)
                    .bind::<diesel::sql_types::Text, _>(name)
                    .load::<RawLastPublisherEntryForPair>(conn)
            })
            .await
            .map_err(|error| error.to_string())?
            .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| {
            tracing::error!(%error, "Historical publisher coverage query failed");
            InfraError::InternalServerError
        })
}

#[cfg(test)]
mod history_tests {
    use super::*;

    fn entry(pair: &str, timestamp: i64, updates: i64) -> RawLastPublisherEntryForPair {
        RawLastPublisherEntryForPair {
            pair_id: pair.into(),
            source: "BINANCE".into(),
            price: BigDecimal::from(timestamp),
            last_updated_timestamp: chrono::DateTime::from_timestamp(timestamp, 0)
                .unwrap()
                .naive_utc(),
            daily_updates: updates,
        }
    }

    #[test]
    fn historical_markets_survive_but_recent_values_and_counts_win() {
        let rows = merge_history(
            vec![entry("OLD/USD", 1, 99), entry("BTC/USD", 1, 3)],
            vec![entry("BTC/USD", 100, 5), entry("NEW/USD", 100, 2)],
        );
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].price, BigDecimal::from(100));
        assert_eq!(rows[0].daily_updates, 5);
        assert_eq!(rows[2].pair_id, "OLD/USD");
        assert_eq!(rows[2].daily_updates, 0);
    }
}
