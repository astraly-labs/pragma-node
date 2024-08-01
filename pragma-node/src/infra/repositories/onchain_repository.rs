use std::collections::HashMap;

use bigdecimal::{BigDecimal, ToPrimitive, Zero};
use deadpool_diesel::postgres::Pool;
use diesel::sql_types::{BigInt, Integer, Numeric, Text, Timestamp, VarChar};
use diesel::{Queryable, QueryableByName, RunQueryDsl};

use moka::future::Cache;
use pragma_common::types::{AggregationMode, DataType, Interval, Network};
use pragma_entities::error::{adapt_infra_error, InfraError};
use pragma_entities::Currency;
use pragma_monitoring::models::SpotEntry;

use crate::handlers::get_onchain::checkpoints::Checkpoint;
use crate::handlers::get_onchain::history::ChunkInterval;
use crate::handlers::get_onchain::publishers::{Publisher, PublisherEntry};
use crate::handlers::get_onchain::OnchainEntry;
use crate::infra::repositories::entry_repository::{
    get_interval_specifier, OHLCEntry, OHLCEntryRaw,
};
use crate::types::timestamp::TimestampParam;
use crate::utils::{
    big_decimal_price_to_hex, convert_via_quote, format_bigdecimal_price, get_decimals_for_pair,
    get_mid_price, normalize_to_decimals,
};

use super::entry_repository::get_decimals;

pub struct RawOnchainData {
    pub price: BigDecimal,
    pub decimal: u32,
    pub sources: Vec<OnchainEntry>,
    pub pair_used: Vec<String>,
}

// Retrieve the onchain table name based on the network and data type.
fn get_table_name(network: Network, data_type: DataType) -> Result<&'static str, InfraError> {
    let table = match (network, data_type) {
        (Network::Sepolia, DataType::SpotEntry) => "spot_entry",
        (Network::Mainnet, DataType::SpotEntry) => "mainnet_spot_entry",
        (Network::Sepolia, DataType::FutureEntry) => "future_entry",
        (Network::Mainnet, DataType::FutureEntry) => "mainnet_future_entry",
        _ => return Err(InfraError::InternalServerError),
    };
    Ok(table)
}

// Retrieve the onchain table name for the OHLC based on network, datatype & interval.
fn get_ohlc_table_name(
    network: Network,
    data_type: DataType,
    interval: Interval,
) -> Result<String, InfraError> {
    let prefix_name = match (network, data_type) {
        (Network::Sepolia, DataType::SpotEntry) => "spot",
        (Network::Mainnet, DataType::SpotEntry) => "mainnet_spot",
        (Network::Sepolia, DataType::FutureEntry) => "future",
        (Network::Mainnet, DataType::FutureEntry) => "mainnet_future",
        _ => return Err(InfraError::InternalServerError),
    };
    let interval_specifier = get_interval_specifier(interval, true)?;
    let table_name = format!("{prefix_name}_{interval_specifier}_candle");
    Ok(table_name)
}

#[derive(Queryable, QueryableByName, Debug)]
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
            price: big_decimal_price_to_hex(&entry.spot_entry.price),
            tx_hash: entry.spot_entry.transaction_hash,
            timestamp: entry.spot_entry.timestamp.and_utc().timestamp() as u64,
        }
    }
}

impl From<&SpotEntryWithAggregatedPrice> for OnchainEntry {
    fn from(entry: &SpotEntryWithAggregatedPrice) -> Self {
        OnchainEntry {
            publisher: entry.spot_entry.publisher.clone(),
            source: entry.spot_entry.source.clone(),
            price: big_decimal_price_to_hex(&entry.spot_entry.price),
            tx_hash: entry.spot_entry.transaction_hash.clone(),
            timestamp: entry.spot_entry.timestamp.and_utc().timestamp() as u64,
        }
    }
}

fn get_aggregation_query(
    aggregation_mode: AggregationMode,
    is_range: bool,
) -> Result<&'static str, InfraError> {
    let query = match aggregation_mode {
        AggregationMode::Mean => "AVG(price) AS aggregated_price",
        AggregationMode::Median if is_range => {
            "(
                SELECT AVG(price)
                FROM (
                    SELECT price
                    FROM FilteredEntries
                    WHERE window_start = FE.window_start
                    ORDER BY price
                    LIMIT 2 - (SELECT COUNT(*) FROM FilteredEntries WHERE window_start = FE.window_start) % 2
                    OFFSET (SELECT (COUNT(*) - 1) / 2 FROM FilteredEntries WHERE window_start = FE.window_start)
                ) AS MedianPrices
            ) AS aggregated_price"
        }
        AggregationMode::Median if !is_range => {
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
    timestamp: TimestampParam,
    chunk_interval: ChunkInterval,
) -> Result<String, InfraError> {
    let table_name = get_table_name(network, DataType::SpotEntry)?;

    let complete_sql_query = match timestamp {
        TimestampParam::Single(ts) => {
            let ts_str = ts.to_string();
            let aggregation_query = get_aggregation_query(aggregation_mode, false)?;
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
                        AND timestamp BETWEEN (to_timestamp({ts_str}) - INTERVAL '{backward_interval}') AND to_timestamp({ts_str})
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
                table_name = table_name,
                backward_interval = chunk_interval.as_sql_interval(),
                aggregation_subquery = aggregation_query,
                ts_str = ts_str
            )
        }
        TimestampParam::Range(range) => {
            let start_ts = range.start().to_string();
            let end_ts = range.end().to_string();
            let aggregation_query = get_aggregation_query(aggregation_mode, true)?;
            format!(
                r#"
                WITH TimeWindows AS (
                    SELECT 
                        generate_series(
                            to_timestamp({start_ts}), 
                            to_timestamp({end_ts}) - INTERVAL '{backward_interval}', 
                            INTERVAL '{backward_interval}'
                        ) AS window_start
                ),
                RankedEntries AS (
                    SELECT 
                        TW.window_start,
                        E.*,
                        ROW_NUMBER() OVER (PARTITION BY E.publisher, E.source, TW.window_start ORDER BY E.timestamp DESC) as rn
                    FROM 
                        {table_name} E
                    JOIN 
                        TimeWindows TW
                    ON 
                        E.timestamp BETWEEN TW.window_start AND (TW.window_start + INTERVAL '{backward_interval}')
                    WHERE 
                        E.pair_id = $1
                ),
                FilteredEntries AS (
                    SELECT *
                    FROM RankedEntries
                    WHERE rn = 1
                ),
                AggregatedPrice AS (
                    SELECT 
                        FE.window_start,
                        {aggregation_subquery}
                    FROM 
                        FilteredEntries FE
                    GROUP BY 
                        FE.window_start
                )
                SELECT DISTINCT 
                    FE.window_start,
                    FE.*,
                    AP.aggregated_price
                FROM 
                    FilteredEntries FE
                JOIN 
                    AggregatedPrice AP
                ON 
                    FE.window_start = AP.window_start
                ORDER BY 
                    FE.window_start DESC, FE.timestamp DESC;
            "#,
                table_name = table_name,
                backward_interval = chunk_interval.as_sql_interval(),
                aggregation_subquery = aggregation_query,
                start_ts = start_ts,
                end_ts = end_ts
            )
        }
    };
    Ok(complete_sql_query)
}

pub fn onchain_pair_exist(existing_pair_list: &[EntryPairId], pair_id: &str) -> bool {
    existing_pair_list.iter().any(|entry| entry == pair_id)
}

fn calculate_rebased_price(
    base_result: (BigDecimal, u32),
    quote_result: (BigDecimal, u32),
) -> Result<(BigDecimal, u32), InfraError> {
    let (base_price, base_decimals) = base_result;
    let (quote_price, quote_decimals) = quote_result;

    if quote_price == BigDecimal::from(0) {
        return Err(InfraError::InternalServerError);
    }

    let (rebase_price, decimals) = if base_decimals < quote_decimals {
        let normalized_base_price =
            normalize_to_decimals(base_price, base_decimals, quote_decimals);
        (
            convert_via_quote(normalized_base_price, quote_price, quote_decimals)?,
            quote_decimals,
        )
    } else {
        let normalized_quote_price =
            normalize_to_decimals(quote_price, quote_decimals, base_decimals);
        (
            convert_via_quote(base_price, normalized_quote_price, base_decimals)?,
            base_decimals,
        )
    };

    Ok((rebase_price, decimals))
}

fn compute_multiple_rebased_price(
    base_alt_result: &mut [AggPriceAndEntries],
    quote_alt_result: &[AggPriceAndEntries],
    alt_pairs: Vec<String>,
    base_alt_decimal: u32,
    quote_alt_decimal: u32,
) -> Result<Vec<RawOnchainData>, InfraError> {
    if quote_alt_result.len() != base_alt_result.len() {
        return Err(InfraError::RoutingError);
    }

    let mut result: Vec<RawOnchainData> = Vec::new();

    for (i, base) in base_alt_result.iter_mut().enumerate() {
        let quote = &quote_alt_result[i];
        let rebased_price = calculate_rebased_price(
            (base.aggregated_price.to_owned(), base_alt_decimal),
            (quote.aggregated_price.to_owned(), quote_alt_decimal),
        )?;
        base.entries.extend(quote.entries.to_owned());
        result.push(RawOnchainData {
            price: rebased_price.0,
            decimal: rebased_price.1,
            sources: base.entries.clone(),
            pair_used: alt_pairs.clone(),
        });
    }

    Ok(result)
}

pub struct OnchainRoutingArguments {
    pub pair_id: String,
    pub network: Network,
    pub timestamp: TimestampParam,
    pub aggregation_mode: AggregationMode,
    pub is_routing: bool,
    pub chunk_interval: ChunkInterval,
}

pub async fn routing(
    onchain_pool: &Pool,
    offchain_pool: &Pool,
    routing_args: OnchainRoutingArguments,
) -> Result<Vec<RawOnchainData>, InfraError> {
    let pair_id = routing_args.pair_id;
    let is_routing = routing_args.is_routing;

    let existing_pair_list = get_existing_pairs(onchain_pool, routing_args.network).await?;
    let mut result: Vec<RawOnchainData> = Vec::new();

    if !is_routing || onchain_pair_exist(&existing_pair_list, &pair_id) {
        let prices_and_entries = get_sources_and_aggregate(
            onchain_pool,
            routing_args.network,
            pair_id.clone(),
            routing_args.timestamp,
            routing_args.aggregation_mode,
            routing_args.chunk_interval,
        )
        .await?;
        let decimal = get_decimals(offchain_pool, &pair_id).await?;
        for row in prices_and_entries {
            result.push(RawOnchainData {
                price: row.aggregated_price,
                decimal,
                sources: row.entries,
                pair_used: vec![pair_id.clone()],
            })
        }
        return Ok(result);
    }
    if !is_routing {
        return Err(InfraError::NotFound);
    }

    let offchain_conn = offchain_pool.get().await.map_err(adapt_infra_error)?;

    let alternative_currencies = offchain_conn
        .interact(Currency::get_abstract_all)
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    // safe unwrap since we construct the pairs string in calling function
    let (base, quote) = pair_id.split_once('/').unwrap();

    for alt_currency in alternative_currencies {
        let base_alt_pair = format!("{}/{}", base, alt_currency);
        let alt_quote_pair = format!("{}/{}", quote, alt_currency);

        if onchain_pair_exist(&existing_pair_list, &base_alt_pair)
            && onchain_pair_exist(&existing_pair_list, &alt_quote_pair)
        {
            let mut base_alt_result = get_sources_and_aggregate(
                onchain_pool,
                routing_args.network,
                base_alt_pair.clone(),
                routing_args.timestamp.clone(),
                routing_args.aggregation_mode,
                routing_args.chunk_interval,
            )
            .await?;
            let base_alt_decimal = get_decimals(offchain_pool, &base_alt_pair).await?;
            let quote_alt_result = get_sources_and_aggregate(
                onchain_pool,
                routing_args.network,
                alt_quote_pair.clone(),
                routing_args.timestamp,
                routing_args.aggregation_mode,
                routing_args.chunk_interval,
            )
            .await?;
            let quote_alt_decimal = get_decimals(offchain_pool, &alt_quote_pair).await?;

            let result = compute_multiple_rebased_price(
                &mut base_alt_result,
                &quote_alt_result,
                vec![base_alt_pair, alt_quote_pair],
                base_alt_decimal,
                quote_alt_decimal,
            );

            return result;
        }
    }
    Err(InfraError::NotFound)
}

#[derive(Debug)]
pub struct AggPriceAndEntries {
    aggregated_price: BigDecimal,
    entries: Vec<OnchainEntry>,
}

fn group_entries_per_aggprice(
    raw_entries: Vec<SpotEntryWithAggregatedPrice>,
) -> Result<Vec<AggPriceAndEntries>, InfraError> {
    if raw_entries.is_empty() {
        return Err(InfraError::NotFound);
    }

    let mut result: Vec<AggPriceAndEntries> = Vec::new();
    let mut curr_agg_price: BigDecimal = BigDecimal::default();
    for entry in raw_entries.iter().rev() {
        if curr_agg_price != entry.aggregated_price {
            result.push(AggPriceAndEntries {
                aggregated_price: entry.aggregated_price.clone(),
                entries: vec![OnchainEntry::from(entry)],
            });
            curr_agg_price = entry.aggregated_price.clone();
        } else {
            result
                .last_mut()
                .unwrap()
                .entries
                .push(OnchainEntry::from(entry));
        }
    }

    Ok(result)
}

// TODO(akhercha): Only works for Spot entries
pub async fn get_sources_and_aggregate(
    pool: &Pool,
    network: Network,
    pair_id: String,
    timestamp: TimestampParam,
    aggregation_mode: AggregationMode,
    chunk_interval: ChunkInterval,
) -> Result<Vec<AggPriceAndEntries>, InfraError> {
    let raw_sql = build_sql_query(network, aggregation_mode, timestamp, chunk_interval)?;

    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let raw_entries = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql)
                .bind::<Text, _>(pair_id)
                .load::<SpotEntryWithAggregatedPrice>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    group_entries_per_aggprice(raw_entries)
}

#[derive(Queryable, QueryableByName)]
struct EntryTimestamp {
    #[diesel(sql_type = Timestamp)]
    pub timestamp: chrono::NaiveDateTime,
}

// TODO(akhercha): Only works for Spot entries
pub async fn get_last_updated_timestamp(
    pool: &Pool,
    network: Network,
    pairs: Vec<String>,
) -> Result<u64, InfraError> {
    let pair_list = format!("('{}')", pairs.join("','"));
    let raw_sql = format!(
        r#"
        SELECT
            timestamp
        FROM
            {}
        WHERE
            pair_id IN {}
        ORDER BY timestamp DESC
        LIMIT 1;
    "#,
        get_table_name(network, DataType::SpotEntry)?,
        pair_list,
    );
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let raw_entry = conn
        .interact(move |conn| diesel::sql_query(raw_sql).load::<EntryTimestamp>(conn))
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    let most_recent_entry = raw_entry.first().ok_or(InfraError::NotFound)?;
    Ok(most_recent_entry.timestamp.and_utc().timestamp() as u64)
}

#[derive(QueryableByName)]
struct VariationEntry {
    #[diesel(sql_type = Numeric)]
    open: BigDecimal,
    #[diesel(sql_type = Numeric)]
    close: BigDecimal,
}

pub async fn get_variations(
    pool: &Pool,
    network: Network,
    pair_id: String,
) -> Result<HashMap<Interval, f32>, InfraError> {
    let intervals = vec![Interval::OneHour, Interval::OneDay, Interval::OneWeek];

    let mut variations = HashMap::new();

    for interval in intervals {
        let ohlc_table_name = get_ohlc_table_name(network, DataType::SpotEntry, interval)?;
        let raw_sql = format!(
            r#"
            WITH recent_entries AS (
                SELECT
                    ohlc_bucket AS time,
                    open,
                    close,
                    ROW_NUMBER() OVER (ORDER BY ohlc_bucket DESC) as rn
                FROM
                    {table_name}
                WHERE
                    pair_id = $1
                ORDER BY
                    time DESC
                LIMIT 2
            )
            SELECT
                open,
                close
            FROM
                recent_entries
            WHERE
                rn IN (1, 2)
            ORDER BY
                rn ASC;
            "#,
            table_name = ohlc_table_name
        );

        let conn = pool.get().await.map_err(adapt_infra_error)?;
        let p = pair_id.clone();
        let raw_entries: Vec<VariationEntry> = conn
            .interact(move |conn| diesel::sql_query(raw_sql).bind::<Text, _>(p).load(conn))
            .await
            .map_err(adapt_infra_error)?
            .map_err(adapt_infra_error)?;

        if raw_entries.len() == 2 {
            let current_open = get_mid_price(&raw_entries[0].open, &raw_entries[0].close);
            let previous_open = get_mid_price(&raw_entries[1].open, &raw_entries[1].close);

            if !previous_open.is_zero() {
                let variation = (current_open - previous_open.clone()) / previous_open;
                if let Some(variation_f32) = variation.to_f32() {
                    variations.insert(interval, variation_f32);
                }
            }
        }
    }

    Ok(variations)
}

#[derive(Queryable, QueryableByName, PartialEq, Debug)]
pub struct EntryPairId {
    #[diesel(sql_type = VarChar)]
    pub pair_id: String,
}

impl PartialEq<str> for EntryPairId {
    fn eq(&self, other: &str) -> bool {
        self.pair_id == other
    }
}

impl PartialEq<String> for EntryPairId {
    fn eq(&self, other: &String) -> bool {
        self.pair_id == other.as_str()
    }
}

// TODO(0xevolve): Only works for Spot entries
pub async fn get_existing_pairs(
    pool: &Pool,
    network: Network,
) -> Result<Vec<EntryPairId>, InfraError> {
    let raw_sql = format!(
        r#"
        SELECT DISTINCT
            pair_id
        FROM
            {table_name};
    "#,
        table_name = get_table_name(network, DataType::SpotEntry)?
    );

    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let raw_entries = conn
        .interact(move |conn| diesel::sql_query(raw_sql).load::<EntryPairId>(conn))
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    Ok(raw_entries)
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
    let table_name = match network {
        Network::Mainnet => "mainnet_spot_checkpoints",
        Network::Sepolia => "spot_checkpoints",
    };
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
        table_name = table_name
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
    let table_name = get_table_name(network, data_type)?;
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

// Only works for Spot for now - since we only store spot entries on chain.
pub async fn get_ohlc(
    pool: &Pool,
    network: Network,
    pair_id: String,
    interval: Interval,
    data_to_retrieve: u64,
) -> Result<Vec<OHLCEntry>, InfraError> {
    let raw_sql = format!(
        r#"
        SELECT
            ohlc_bucket AS time,
            open,
            high,
            low,
            close
        FROM
            {table_name}
        WHERE
            pair_id = $1
        ORDER BY
            time DESC
        LIMIT {data_to_retrieve};
        "#,
        table_name = get_ohlc_table_name(network, DataType::SpotEntry, interval)?,
    );

    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let raw_entries = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql)
                .bind::<diesel::sql_types::Text, _>(pair_id)
                .load::<OHLCEntryRaw>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    let entries: Vec<OHLCEntry> = raw_entries
        .into_iter()
        .map(|raw_entry| OHLCEntry {
            time: raw_entry.time,
            open: raw_entry.open,
            high: raw_entry.high,
            low: raw_entry.low,
            close: raw_entry.close,
        })
        .collect();

    Ok(entries)
}
