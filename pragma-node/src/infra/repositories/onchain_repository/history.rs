use std::collections::HashMap;

use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDateTime};
use deadpool_diesel::postgres::Pool;
use diesel::{RunQueryDsl, prelude::QueryableByName};
use moka::future::Cache;
use pragma_entities::models::entries::timestamp::TimestampRange;
use serde::Serialize;

use diesel::Connection;
use pragma_common::starknet::StarknetNetwork;
use pragma_common::{InstrumentType, Interval, Pair};
use pragma_entities::TimestampError;
use pragma_entities::error::InfraError;

use super::entry::{get_existing_pairs, onchain_pair_exist};
use super::{get_onchain_aggregate_table_name, get_onchain_decimals};
use crate::constants::currencies::ABSTRACT_CURRENCIES;
use crate::infra::rpc::RpcClients;
use crate::utils::{convert_via_quote, normalize_to_decimals};

/// Query the onchain database for historical entries and if entries
/// are found, query the offchain database to get the pair decimals.
#[allow(clippy::implicit_hasher)]
pub async fn get_historical_entries_and_decimals(
    onchain_pool: &Pool,
    network: StarknetNetwork,
    pair: &Pair,
    timestamp_range: &TimestampRange,
    chunk_interval: Interval,
    decimals_cache: &Cache<StarknetNetwork, HashMap<String, u32>>,
    rpc_clients: &RpcClients,
) -> Result<(Vec<HistoricalEntryRaw>, u32), InfraError> {
    let raw_entries: Vec<HistoricalEntryRaw> = get_historical_aggregated_entries(
        onchain_pool,
        network,
        pair,
        timestamp_range,
        chunk_interval,
    )
    .await?;

    if raw_entries.is_empty() {
        return Err(InfraError::EntryNotFound(pair.to_pair_id()));
    }

    let decimals = get_onchain_decimals(decimals_cache, rpc_clients, network, pair).await?;

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
/// NOTE: Only works for `SpotEntry` at the moment, `DataType` is hard coded.
async fn get_historical_aggregated_entries(
    pool: &Pool,
    network: StarknetNetwork,
    pair: &Pair,
    timestamp: &TimestampRange,
    chunk_interval: Interval,
) -> Result<Vec<HistoricalEntryRaw>, InfraError> {
    let (start_timestamp, end_timestamp) = {
        let range = timestamp.clone().0;
        (*range.start(), *range.end())
    };

    let raw_sql = format!(
        r"
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
        ",
        table_name =
            get_onchain_aggregate_table_name(network, InstrumentType::Spot, chunk_interval)?,
    );

    let pair_id = pair.to_string();

    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;
    let raw_entries = conn
        .interact(move |conn| {
            conn.transaction(|conn| {
                diesel::sql_query(raw_sql)
                    .bind::<diesel::sql_types::Text, _>(&pair_id)
                    .bind::<diesel::sql_types::BigInt, _>(start_timestamp)
                    .bind::<diesel::sql_types::BigInt, _>(end_timestamp)
                    .load::<HistoricalEntryRaw>(conn)
            })
        })
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    Ok(raw_entries)
}

/// Retry to get the onchain historical entries by finding
/// an alternative route.
///
/// TODO: This code is very similar to the one in [`entry_repository`] ;
///       once we have proper E2E tests, we should try to merge the code.
/// NOTE: We let the possibility to try 1min intervals but they rarely works.
/// Entries rarely align perfectly, causing insufficient data for routing.
#[allow(clippy::implicit_hasher)]
pub async fn retry_with_routing(
    onchain_pool: &Pool,
    network: StarknetNetwork,
    pair: &Pair,
    timestamp_range: &TimestampRange,
    chunk_interval: Interval,
    decimals_cache: &Cache<StarknetNetwork, HashMap<String, u32>>,
    rpc_clients: &RpcClients,
) -> Result<(Vec<HistoricalEntryRaw>, u32), InfraError> {
    let existing_pairs = get_existing_pairs(onchain_pool, network).await?;
    let mut routing_attempts = Vec::new();

    for alt_currency in ABSTRACT_CURRENCIES {
        let base_alt_pair = Pair::from((pair.base.clone(), alt_currency.to_string()));
        let alt_quote_pair = Pair::from((pair.quote.clone(), alt_currency.to_string()));
        let base_alt_pair_str = base_alt_pair.to_string();
        let alt_quote_pair_str = alt_quote_pair.to_string();

        // Check if both required pairs exist
        let base_alt_exists = onchain_pair_exist(&existing_pairs, &base_alt_pair_str);
        let alt_quote_exists = onchain_pair_exist(&existing_pairs, &alt_quote_pair_str);

        if !base_alt_exists || !alt_quote_exists {
            routing_attempts.push(format!(
                "Route via {alt_currency}: base pair '{base_alt_pair_str}' exists: {base_alt_exists}, quote pair '{alt_quote_pair_str}' exists: {alt_quote_exists}",                
            ));
            continue;
        }

        // Both pairs exist, try to get their historical entries
        let base_alt_result = get_historical_entries_and_decimals(
            onchain_pool,
            network,
            &base_alt_pair,
            timestamp_range,
            chunk_interval,
            decimals_cache,
            rpc_clients,
        )
        .await;

        if let Err(e) = &base_alt_result {
            routing_attempts.push(format!(
                "Route via {alt_currency}: failed to get history for '{base_alt_pair_str}': {e}",
            ));
            continue;
        }

        let alt_quote_result = get_historical_entries_and_decimals(
            onchain_pool,
            network,
            &alt_quote_pair,
            timestamp_range,
            chunk_interval,
            decimals_cache,
            rpc_clients,
        )
        .await;

        if let Err(e) = &alt_quote_result {
            routing_attempts.push(format!(
                "Route via {alt_currency}: failed to get history for '{alt_quote_pair_str}': {e}",
            ));
            continue;
        }

        let base_alt_result = base_alt_result.unwrap();
        let alt_quote_result = alt_quote_result.unwrap();

        if base_alt_result.0.len() != alt_quote_result.0.len() {
            routing_attempts.push(format!(
                "Route via {alt_currency}: mismatched entries count: {} vs {}",
                base_alt_result.0.len(),
                alt_quote_result.0.len()
            ));
            continue;
        }

        return calculate_rebased_prices(base_alt_result, alt_quote_result);
    }

    // Construct detailed error message
    let attempts_info = if routing_attempts.is_empty() {
        "No routing pairs found".to_string()
    } else {
        format!("Attempted routes:\n- {}", routing_attempts.join("\n- "))
    };

    Err(InfraError::RoutingError(format!(
        "{}; {attempts_info}",
        pair.to_pair_id(),
    )))
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

/// Wrapper around the [`normalize_to_decimals`] function. It will be applied to all
/// entries `median_price`.
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

/// Wrapper around the [`convert_via_quote`] function. It will be applied to all
/// `normalized_entries` median price.
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

/// Given two entries, determine what should be the resulted `pair_id`, timestamp
/// & sources. Returns the new entry afterwards.
fn combine_entries(
    base_entry: &HistoricalEntryRaw,
    quote_entry: &HistoricalEntryRaw,
    converted_price: BigDecimal,
) -> Result<HistoricalEntryRaw, InfraError> {
    let max_timestamp = std::cmp::max(
        base_entry.timestamp.and_utc().timestamp(),
        quote_entry.timestamp.and_utc().timestamp(),
    );
    let num_sources = std::cmp::max(
        base_entry.nb_sources_aggregated,
        quote_entry.nb_sources_aggregated,
    );
    let new_timestamp = DateTime::from_timestamp(max_timestamp, 0)
        .ok_or(InfraError::InvalidTimestamp(
            TimestampError::ToDatetimeErrorI64(max_timestamp),
        ))?
        .naive_utc();

    let base_pair = Pair::from(base_entry.pair_id.clone());
    let quote_pair = Pair::from(quote_entry.pair_id.clone());

    Ok(HistoricalEntryRaw {
        pair_id: Pair::create_routed_pair(&base_pair, &quote_pair).to_string(),
        timestamp: new_timestamp,
        median_price: converted_price,
        nb_sources_aggregated: num_sources,
    })
}
