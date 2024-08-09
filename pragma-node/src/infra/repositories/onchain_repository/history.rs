use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDateTime};
use deadpool_diesel::postgres::Pool;
use diesel::{prelude::QueryableByName, RunQueryDsl};

use pragma_common::types::{DataType, Interval, Network};
use pragma_entities::error::{adapt_infra_error, InfraError};
use pragma_entities::Currency;
use serde::Serialize;

use crate::infra::repositories::entry_repository::get_decimals;
use crate::types::timestamp::TimestampRange;
use crate::utils::{convert_via_quote, normalize_to_decimals};

use super::entry::{get_existing_pairs, onchain_pair_exist};
use super::get_onchain_aggregate_table_name;

/// Query the onchain database for historical entries and if entries
/// are found, query the offchain database to get the pair decimals.
pub async fn get_historical_entries_and_decimals(
    onchain_pool: &Pool,
    offchain_pool: &Pool,
    network: &Network,
    pair_id: String,
    timestamp_range: &TimestampRange,
    chunk_interval: &Interval,
) -> Result<(Vec<HistoricalEntryRaw>, u32), InfraError> {
    let raw_entries: Vec<HistoricalEntryRaw> = get_historical_aggregated_entries(
        onchain_pool,
        network,
        pair_id.clone(),
        timestamp_range,
        chunk_interval,
    )
    .await?;

    if raw_entries.is_empty() {
        return Err(InfraError::NotFound);
    }

    let decimals = get_decimals(offchain_pool, &pair_id).await?;
    Ok((raw_entries, decimals))
}

#[derive(Serialize, QueryableByName, Clone, Debug)]
pub struct HistoricalEntryRaw {
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    pub pair_id: String,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub timestamp: NaiveDateTime,
    #[diesel(sql_type = diesel::sql_types::Numeric)]
    pub median_price: BigDecimal,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub nb_sources_aggregated: i64,
}

/// Returns the historical entries for a pair and the selected interval.
/// NOTE: Only works for SpotEntry at the moment, DataType is hard coded.
async fn get_historical_aggregated_entries(
    pool: &Pool,
    network: &Network,
    pair_id: String,
    timestamp: &TimestampRange,
    chunk_interval: &Interval,
) -> Result<Vec<HistoricalEntryRaw>, InfraError> {
    let (start_timestamp, end_timestamp) = {
        let range = timestamp.clone().0;
        (*range.start(), *range.end())
    };

    let raw_sql = format!(
        r#"
        SELECT
            pair_id,
            bucket AS timestamp,
            median_price,
            num_sources AS nb_sources_aggregated
        FROM
            {table_name}
        WHERE
            pair_id = $1
            AND bucket >= to_timestamp($2)
            AND bucket <= to_timestamp($3)
        ORDER BY
            bucket ASC
        "#,
        table_name =
            get_onchain_aggregate_table_name(network, &DataType::SpotEntry, chunk_interval)?,
    );

    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let raw_entries = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql)
                .bind::<diesel::sql_types::Text, _>(&pair_id)
                .bind::<diesel::sql_types::BigInt, _>(start_timestamp)
                .bind::<diesel::sql_types::BigInt, _>(end_timestamp)
                .load::<HistoricalEntryRaw>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    Ok(raw_entries)
}

/// Retry to get the onchain historical entries by finding
/// an alternative route.
/// TODO: This code is very similar to the one in [entry_repository] ;
///       once we have proper E2E tests, we should try to merge the code.
pub async fn retry_with_routing(
    onchain_pool: &Pool,
    offchain_pool: &Pool,
    network: &Network,
    pair_id: String,
    timestamp_range: &TimestampRange,
    chunk_interval: &Interval,
) -> Result<(Vec<HistoricalEntryRaw>, u32), InfraError> {
    let [base, quote]: [&str; 2] = pair_id
        .split('/')
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| InfraError::InternalServerError)?;

    let offchain_conn = offchain_pool.get().await.map_err(adapt_infra_error)?;
    let alternative_currencies = offchain_conn
        .interact(Currency::get_abstract_all)
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    let existing_pairs = get_existing_pairs(onchain_pool, network).await?;

    for alt_currency in alternative_currencies {
        let base_alt_pair = format!("{}/{}", base, alt_currency);
        let alt_quote_pair = format!("{}/{}", quote, alt_currency);

        if onchain_pair_exist(&existing_pairs, &base_alt_pair)
            && onchain_pair_exist(&existing_pairs, &alt_quote_pair)
        {
            let base_alt_result = get_historical_entries_and_decimals(
                onchain_pool,
                offchain_pool,
                network,
                base_alt_pair,
                timestamp_range,
                chunk_interval,
            )
            .await?;
            let alt_quote_result = get_historical_entries_and_decimals(
                onchain_pool,
                offchain_pool,
                network,
                alt_quote_pair,
                timestamp_range,
                chunk_interval,
            )
            .await?;

            if base_alt_result.0.len() != alt_quote_result.0.len() {
                continue;
            }

            return calculate_rebased_prices(base_alt_result, alt_quote_result);
        }
    }

    Err(InfraError::NotFound)
}

/// Given two vector of entries, compute a new vector containing the routed prices.
fn calculate_rebased_prices(
    base_result: (Vec<HistoricalEntryRaw>, u32),
    quote_result: (Vec<HistoricalEntryRaw>, u32),
) -> Result<(Vec<HistoricalEntryRaw>, u32), InfraError> {
    let (base_entries, base_decimals) = base_result;
    let (quote_entries, quote_decimals) = quote_result;

    let (rebased_entries, decimals) = if base_decimals < quote_decimals {
        let normalized_base_entries =
            normalize_entries_to_decimals(base_entries, base_decimals, quote_decimals);
        (
            convert_entries_via_quote(normalized_base_entries, quote_entries, quote_decimals)?,
            quote_decimals,
        )
    } else {
        let normalized_quote_entries =
            normalize_entries_to_decimals(quote_entries, quote_decimals, base_decimals);
        (
            convert_entries_via_quote(base_entries, normalized_quote_entries, base_decimals)?,
            base_decimals,
        )
    };

    Ok((rebased_entries, decimals))
}

/// Wrapper around the [normalize_to_decimals] function. It will be applied to all
/// entries median_price.
fn normalize_entries_to_decimals(
    entries: Vec<HistoricalEntryRaw>,
    base_decimals: u32,
    quote_decimals: u32,
) -> Vec<HistoricalEntryRaw> {
    entries
        .into_iter()
        .map(|mut entry| {
            entry.median_price =
                normalize_to_decimals(entry.median_price, base_decimals, quote_decimals);
            entry
        })
        .collect()
}

/// Wrapper around the [convert_via_quote] function. It will be applied to all
/// normalized_entries median_price.
fn convert_entries_via_quote(
    normalized_entries: Vec<HistoricalEntryRaw>,
    quote_entries: Vec<HistoricalEntryRaw>,
    decimals: u32,
) -> Result<Vec<HistoricalEntryRaw>, InfraError> {
    normalized_entries
        .into_iter()
        .zip(quote_entries)
        .map(|(base_entry, quote_entry)| {
            let converted_price = convert_via_quote(
                base_entry.median_price.clone(),
                quote_entry.median_price.clone(),
                decimals,
            )?;
            combine_entries(&base_entry, &quote_entry, converted_price)
        })
        .collect()
}

/// Given two entries, determine what should be the resulted pair_id, timestamp
/// & sources. Returns the new entry afterwards.
fn combine_entries(
    base_entry: &HistoricalEntryRaw,
    quote_entry: &HistoricalEntryRaw,
    converted_price: BigDecimal,
) -> Result<HistoricalEntryRaw, InfraError> {
    let new_pair_id = construct_new_pair_id(&base_entry.pair_id, &quote_entry.pair_id)?;
    let min_timestamp = std::cmp::max(
        base_entry.timestamp.and_utc().timestamp(),
        quote_entry.timestamp.and_utc().timestamp(),
    );
    let num_sources = std::cmp::max(
        base_entry.nb_sources_aggregated,
        quote_entry.nb_sources_aggregated,
    );
    let new_timestamp = DateTime::from_timestamp(min_timestamp, 0)
        .ok_or(InfraError::InvalidTimestamp(format!(
            "Cannot convert to DateTime: {min_timestamp}"
        )))?
        .naive_utc();

    Ok(HistoricalEntryRaw {
        pair_id: new_pair_id,
        timestamp: new_timestamp,
        median_price: converted_price,
        nb_sources_aggregated: num_sources,
    })
}

fn construct_new_pair_id(base_pair_id: &str, quote_pair_id: &str) -> Result<String, InfraError> {
    // Extract base currency from base_entry pair_id
    let base_currency = base_pair_id
        .split('/')
        .next()
        .ok_or_else(|| InfraError::InternalServerError)?;

    // Extract quote currency from quote_entry pair_id
    let quote_currency = quote_pair_id
        .split('/')
        .next()
        .ok_or_else(|| InfraError::InternalServerError)?;

    Ok(format!("{}/{}", base_currency, quote_currency))
}
