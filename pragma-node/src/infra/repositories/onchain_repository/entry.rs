use std::collections::HashMap;

use bigdecimal::{BigDecimal, ToPrimitive, Zero};
use deadpool_diesel::postgres::Pool;
use diesel::sql_types::{Numeric, Text, Timestamp, VarChar};
use diesel::{Queryable, QueryableByName, RunQueryDsl};
use moka::future::Cache;

use pragma_common::Pair;
use pragma_common::{AggregationMode, InstrumentType, Interval, starknet::StarknetNetwork};
use pragma_entities::error::InfraError;
use pragma_monitoring::models::SpotEntry;

use crate::constants::currencies::ABSTRACT_CURRENCIES;
use crate::handlers::onchain::get_entry::OnchainEntry;
use crate::infra::rpc::RpcClients;
use crate::utils::{
    big_decimal_price_to_hex, convert_via_quote, get_mid_price, normalize_to_decimals,
};
use diesel::Connection;

use super::{get_onchain_decimals, get_onchain_ohlc_table_name, get_onchain_table_name};

// Means that we only consider the entries for the last hour when computing the aggregation &
// retrieving the sources.
pub const ENTRIES_BACKWARD_INTERVAL: &str = "1 hour";

#[derive(Debug)]
pub struct OnchainEntryArguments {
    pub pair_id: String,
    pub network: StarknetNetwork,
    pub timestamp: u64,
    pub aggregation_mode: AggregationMode,
    pub is_routing: bool,
}

pub struct RawOnchainData {
    pub price: BigDecimal,
    pub decimal: u32,
    pub sources: Vec<OnchainEntry>,
    pub pair_used: Vec<String>,
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
        Self {
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
        Self {
            publisher: entry.spot_entry.publisher.clone(),
            source: entry.spot_entry.source.clone(),
            price: big_decimal_price_to_hex(&entry.spot_entry.price),
            tx_hash: entry.spot_entry.transaction_hash.clone(),
            timestamp: entry.spot_entry.timestamp.and_utc().timestamp() as u64,
        }
    }
}

#[allow(clippy::implicit_hasher)]
pub async fn routing(
    onchain_pool: &Pool,
    routing_args: OnchainEntryArguments,
    rpc_clients: &RpcClients,
    decimals_cache: &Cache<StarknetNetwork, HashMap<String, u32>>,
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
        )
        .await?;
        if !prices_and_entries.is_empty() {
            let pair = Pair::from(pair_id.clone());
            let decimal =
                get_onchain_decimals(decimals_cache, rpc_clients, routing_args.network, &pair)
                    .await?;
            for row in prices_and_entries {
                result.push(RawOnchainData {
                    price: row.aggregated_price,
                    decimal,
                    sources: row.entries,
                    pair_used: vec![pair_id.clone()],
                });
            }
            return Ok(result);
        }
    }
    if !is_routing {
        return Err(InfraError::EntryNotFound(pair_id));
    }

    // safe unwrap since we construct the pairs string in calling function
    let (base, quote) = pair_id.split_once('/').unwrap();

    for alt_currency in ABSTRACT_CURRENCIES {
        let base_alt_pair = format!("{base}/{alt_currency}");
        let alt_quote_pair = format!("{quote}/{alt_currency}");

        if onchain_pair_exist(&existing_pair_list, &base_alt_pair)
            && onchain_pair_exist(&existing_pair_list, &alt_quote_pair)
        {
            let mut base_alt_result = get_sources_and_aggregate(
                onchain_pool,
                routing_args.network,
                base_alt_pair.clone(),
                routing_args.timestamp,
                routing_args.aggregation_mode,
            )
            .await?;
            let base_alt_decimal = get_onchain_decimals(
                decimals_cache,
                rpc_clients,
                routing_args.network,
                &Pair::from(base_alt_pair.clone()),
            )
            .await?;
            let quote_alt_result = get_sources_and_aggregate(
                onchain_pool,
                routing_args.network,
                alt_quote_pair.clone(),
                routing_args.timestamp,
                routing_args.aggregation_mode,
            )
            .await?;
            let quote_alt_decimal = get_onchain_decimals(
                decimals_cache,
                rpc_clients,
                routing_args.network,
                &Pair::from(alt_quote_pair.clone()),
            )
            .await?;

            if quote_alt_result.len() != base_alt_result.len() {
                return Err(InfraError::RoutingError(pair_id));
            }

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
    Err(InfraError::RoutingError(pair_id))
}

fn build_sql_query(
    network: StarknetNetwork,
    aggregation_mode: AggregationMode,
    timestamp: u64,
) -> Result<String, InfraError> {
    let table_name = get_onchain_table_name(network, InstrumentType::Spot);

    let complete_sql_query = {
        let aggregation_query = get_aggregation_subquery(aggregation_mode)?;
        format!(
            r"
                WITH RankedEntries AS (
                    SELECT 
                        *,
                        ROW_NUMBER() OVER (PARTITION BY publisher, source ORDER BY timestamp DESC) as rn
                    FROM 
                        {table_name}
                    WHERE 
                        pair_id = $1
                        AND timestamp BETWEEN (to_timestamp({timestamp}) - INTERVAL '{ENTRIES_BACKWARD_INTERVAL}') AND to_timestamp({timestamp})
                ),
                FilteredEntries AS (
                    SELECT *
                    FROM RankedEntries
                    WHERE rn = 1
                ),
                AggregatedPrice AS (
                    SELECT {aggregation_query}
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
            ",
        )
    };
    Ok(complete_sql_query)
}

fn get_aggregation_subquery(aggregation_mode: AggregationMode) -> Result<&'static str, InfraError> {
    let query = match aggregation_mode {
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

#[derive(Debug)]
pub struct AggPriceAndEntries {
    aggregated_price: BigDecimal,
    entries: Vec<OnchainEntry>,
}

// TODO(akhercha): Only works for Spot entries
pub async fn get_sources_and_aggregate(
    pool: &Pool,
    network: StarknetNetwork,
    pair_id: String,
    timestamp: u64,
    aggregation_mode: AggregationMode,
) -> Result<Vec<AggPriceAndEntries>, InfraError> {
    let raw_sql = build_sql_query(network, aggregation_mode, timestamp)?;

    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;
    let raw_entries = conn
        .interact(move |conn| {
            conn.transaction(|conn| {
                diesel::sql_query(raw_sql)
                    .bind::<Text, _>(pair_id)
                    .load::<SpotEntryWithAggregatedPrice>(conn)
            })
        })
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    Ok(group_entries_per_aggprice(raw_entries))
}

fn group_entries_per_aggprice(
    raw_entries: Vec<SpotEntryWithAggregatedPrice>,
) -> Vec<AggPriceAndEntries> {
    let mut result: Vec<AggPriceAndEntries> = Vec::new();
    let mut curr_agg_price: BigDecimal = BigDecimal::default();
    for entry in raw_entries.iter().rev() {
        // TODO: Remove this unsafe unwrap, probably by checking the vec size first
        if curr_agg_price == entry.aggregated_price {
            result
                .last_mut()
                .unwrap()
                .entries
                .push(OnchainEntry::from(entry));
        } else {
            result.push(AggPriceAndEntries {
                aggregated_price: entry.aggregated_price.clone(),
                entries: vec![OnchainEntry::from(entry)],
            });
            curr_agg_price = entry.aggregated_price.clone();
        }
    }

    result
}

fn compute_multiple_rebased_price(
    base_alt_result: &mut [AggPriceAndEntries],
    quote_alt_result: &[AggPriceAndEntries],
    alt_pairs: Vec<String>,
    base_alt_decimal: u32,
    quote_alt_decimal: u32,
) -> Result<Vec<RawOnchainData>, InfraError> {
    let mut result: Vec<RawOnchainData> = Vec::new();

    for (i, base) in base_alt_result.iter_mut().enumerate() {
        let quote = &quote_alt_result[i];
        let rebased_price = calculate_rebased_price(
            (base.aggregated_price.clone(), base_alt_decimal),
            (quote.aggregated_price.clone(), quote_alt_decimal),
        )?;
        base.entries.extend(quote.entries.clone());
        result.push(RawOnchainData {
            price: rebased_price.0,
            decimal: rebased_price.1,
            sources: base.entries.clone(),
            pair_used: alt_pairs.clone(),
        });
    }

    Ok(result)
}

#[derive(Queryable, QueryableByName)]
struct EntryTimestamp {
    #[diesel(sql_type = Timestamp)]
    pub timestamp: chrono::NaiveDateTime,
}

pub async fn get_last_updated_timestamp(
    pool: &Pool,
    network: StarknetNetwork,
    pairs: Vec<String>,
) -> Result<u64, InfraError> {
    let pair_list = format!("('{}')", pairs.join("','"));
    let raw_sql = format!(
        r"
        SELECT
            timestamp
        FROM
            {}
        WHERE
            pair_id IN {}
        ORDER BY timestamp DESC
        LIMIT 1;
    ",
        get_onchain_table_name(network, InstrumentType::Spot),
        pair_list,
    );
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;
    let raw_entry = conn
        .interact(move |conn| {
            conn.transaction(|conn| diesel::sql_query(raw_sql).load::<EntryTimestamp>(conn))
        })
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    let most_recent_entry = raw_entry
        .first()
        .ok_or(InfraError::EntryNotFound(pair_list))?;

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
    network: StarknetNetwork,
    pair_id: String,
) -> Result<HashMap<Interval, f32>, InfraError> {
    let intervals = vec![Interval::OneHour, Interval::OneDay, Interval::OneWeek];

    let mut variations = HashMap::new();

    for interval in intervals {
        let ohlc_table_name = get_onchain_ohlc_table_name(network, InstrumentType::Spot, interval)?;
        let raw_sql = format!(
            r"
            WITH recent_entries AS (
                SELECT
                    ohlc_bucket AS time,
                    open,
                    close,
                    ROW_NUMBER() OVER (ORDER BY ohlc_bucket DESC) as rn
                FROM
                    {ohlc_table_name}
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
            ",
        );

        let conn = pool.get().await.map_err(InfraError::DbPoolError)?;
        let p = pair_id.clone();
        let raw_entries: Vec<VariationEntry> = conn
            .interact(move |conn| {
                conn.transaction(|conn| diesel::sql_query(raw_sql).bind::<Text, _>(p).load(conn))
            })
            .await
            .map_err(InfraError::DbInteractionError)?
            .map_err(InfraError::DbResultError)?;

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

#[derive(Queryable, QueryableByName, PartialEq, Eq, Debug)]
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

pub fn onchain_pair_exist(existing_pair_list: &[EntryPairId], pair_id: &str) -> bool {
    existing_pair_list.iter().any(|entry| entry == pair_id)
}

// TODO(0xevolve): Only works for Spot entries
pub async fn get_existing_pairs(
    pool: &Pool,
    network: StarknetNetwork,
) -> Result<Vec<EntryPairId>, InfraError> {
    let raw_sql = format!(
        r"
        SELECT DISTINCT
            pair_id
        FROM
            {table_name};
    ",
        table_name = get_onchain_table_name(network, InstrumentType::Spot)
    );

    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;
    let raw_entries = conn
        .interact(move |conn| {
            conn.transaction(|conn| diesel::sql_query(raw_sql).load::<EntryPairId>(conn))
        })
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    Ok(raw_entries)
}
