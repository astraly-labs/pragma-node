use std::collections::HashMap;

use bigdecimal::{BigDecimal, ToPrimitive, Zero};
use deadpool_diesel::postgres::Pool;
use diesel::sql_types::{Numeric, Text, Timestamp, VarChar};
use diesel::{Queryable, QueryableByName, RunQueryDsl};

use pragma_common::types::{AggregationMode, DataType, Interval, Network};
use pragma_entities::error::{adapt_infra_error, InfraError};
use pragma_entities::Currency;
use pragma_monitoring::models::SpotEntry;

use crate::handlers::onchain::get_entry::OnchainEntry;
use crate::utils::{
    big_decimal_price_to_hex, convert_via_quote, get_mid_price, normalize_to_decimals,
};

use super::{get_onchain_ohlc_table_name, get_onchain_table_name};

use crate::infra::repositories::entry_repository::get_decimals;

// Means that we only consider the entries for the last hour when computing the aggregation &
// retrieving the sources.
pub const ENTRIES_BACKWARD_INTERVAL: &str = "1 hour";

#[derive(Debug)]
pub struct OnchainRoutingArguments {
    pub pair_id: String,
    pub network: Network,
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
        )
        .await?;
        if !prices_and_entries.is_empty() {
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
                routing_args.timestamp,
                routing_args.aggregation_mode,
            )
            .await?;
            let base_alt_decimal = get_decimals(offchain_pool, &base_alt_pair).await?;
            let quote_alt_result = get_sources_and_aggregate(
                onchain_pool,
                routing_args.network,
                alt_quote_pair.clone(),
                routing_args.timestamp,
                routing_args.aggregation_mode,
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

fn build_sql_query(
    network: Network,
    aggregation_mode: AggregationMode,
    timestamp: u64,
) -> Result<String, InfraError> {
    let table_name = get_onchain_table_name(network, DataType::SpotEntry)?;

    let complete_sql_query = {
        let aggregation_query = get_aggregation_subquery(aggregation_mode)?;
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
                        AND timestamp BETWEEN (to_timestamp({timestamp}) - INTERVAL '{ENTRIES_BACKWARD_INTERVAL}') AND to_timestamp({timestamp})
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
            aggregation_subquery = aggregation_query,
            timestamp = timestamp
        )
    };
    Ok(complete_sql_query)
}

fn get_aggregation_subquery(aggregation_mode: AggregationMode) -> Result<&'static str, InfraError> {
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
    network: Network,
    pair_id: String,
    timestamp: u64,
    aggregation_mode: AggregationMode,
) -> Result<Vec<AggPriceAndEntries>, InfraError> {
    let raw_sql = build_sql_query(network, aggregation_mode, timestamp)?;

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

fn group_entries_per_aggprice(
    raw_entries: Vec<SpotEntryWithAggregatedPrice>,
) -> Result<Vec<AggPriceAndEntries>, InfraError> {
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

#[derive(Queryable, QueryableByName)]
struct EntryTimestamp {
    #[diesel(sql_type = Timestamp)]
    pub timestamp: chrono::NaiveDateTime,
}

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
        get_onchain_table_name(network, DataType::SpotEntry)?,
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
        let ohlc_table_name = get_onchain_ohlc_table_name(network, DataType::SpotEntry, interval)?;
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

pub fn onchain_pair_exist(existing_pair_list: &[EntryPairId], pair_id: &str) -> bool {
    existing_pair_list.iter().any(|entry| entry == pair_id)
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
        table_name = get_onchain_table_name(network, DataType::SpotEntry)?
    );

    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let raw_entries = conn
        .interact(move |conn| diesel::sql_query(raw_sql).load::<EntryPairId>(conn))
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    Ok(raw_entries)
}
