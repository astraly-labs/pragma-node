use std::collections::HashMap;

use bigdecimal::BigDecimal;
use chrono::{DateTime, Duration, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::sql_types::{BigInt, Integer, Numeric, Text, Timestamptz, VarChar};
use diesel::{Queryable, QueryableByName, RunQueryDsl};

use pragma_common::types::{AggregationMode, DataType, Interval, Network};
use pragma_entities::error::{adapt_infra_error, InfraError};
use pragma_monitoring::models::SpotEntry;

use crate::handlers::entries::utils::get_decimals_for_pair;
use crate::handlers::entries::{Checkpoint, OnchainEntry, Publisher, PublisherEntry};
use crate::utils::format_bigdecimal_price;

use super::entry_repository::OHLCEntry;

const BACKWARD_TIMESTAMP_INTERVAL: &str = "1 hour";

fn get_table_name(network: Network, data_type: DataType) -> &'static str {
    match (network, data_type) {
        (Network::Testnet, DataType::SpotEntry) => "spot_entry",
        (Network::Mainnet, DataType::SpotEntry) => "mainnet_spot_entry",
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
            timestamp: entry.spot_entry.timestamp.timestamp() as u64,
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
    #[diesel(sql_type = Timestamptz)]
    pub timestamp: DateTime<Utc>,
}

// TODO(akhercha): Only works for Spot entries
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
    Ok(most_recent_entry.timestamp.timestamp() as u64)
}

#[derive(Queryable, QueryableByName)]
struct RawCheckpoint {
    #[diesel(sql_type = VarChar)]
    pub transaction_hash: String,
    #[diesel(sql_type = Numeric)]
    pub price: BigDecimal,
    #[diesel(sql_type = Timestamptz)]
    pub timestamp: DateTime<Utc>,
    #[diesel(sql_type = VarChar)]
    pub sender_address: String,
}

impl RawCheckpoint {
    pub fn to_checkpoint(&self, decimals: u32) -> Checkpoint {
        Checkpoint {
            tx_hash: self.transaction_hash.clone(),
            price: format_bigdecimal_price(self.price.clone(), decimals),
            timestamp: self.timestamp.timestamp() as u64,
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
        Network::Testnet => "spot_checkpoints",
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
        Network::Testnet => "testnet_address",
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
    #[diesel(sql_type = Timestamptz)]
    pub last_updated_timestamp: DateTime<Utc>,
}

impl RawLastPublisherEntryForPair {
    pub fn to_publisher_entry(&self, currencies: &HashMap<String, BigDecimal>) -> PublisherEntry {
        let decimals = get_decimals_for_pair(currencies, &self.pair_id);
        PublisherEntry {
            pair_id: self.pair_id.clone(),
            last_updated_timestamp: self.last_updated_timestamp.timestamp() as u64,
            price: format_bigdecimal_price(self.price.clone(), decimals),
            source: self.source.clone(),
            decimals,
        }
    }
}

#[derive(Debug, Queryable, QueryableByName)]
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
) -> Result<HashMap<String, RawPublisherUpdates>, InfraError> {
    let publishers_list = publishers_names.join("','");
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

    let updates = updates
        .into_iter()
        .map(|update| (update.publisher.clone(), update))
        .collect();

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
) -> Result<Vec<Publisher>, InfraError> {
    let table_name = get_table_name(network, data_type);
    let publisher_names = publishers.iter().map(|p| p.name.clone()).collect();

    let updates = get_all_publishers_updates(pool, table_name, publisher_names).await?;
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

// --- onchain OHLC ---

pub async fn get_ohlc(
    ohlc_data: &mut Vec<OHLCEntry>,
    pool: &Pool,
    network: Network,
    pair_id: String,
    interval: Interval,
    data_to_retrieve: i64,
) -> Result<(), InfraError> {
    let now = Utc::now();
    let aligned_current_timestamp = interval.align_timestamp(now);
    let start_timestamp = if data_to_retrieve > 1 {
        aligned_current_timestamp
            - Duration::minutes(interval.to_minutes() * (data_to_retrieve * 10))
    } else {
        aligned_current_timestamp
    };

    let entries = get_entries_from_timestamp(pool, network, &pair_id, start_timestamp).await?;
    update_ohlc_data(
        ohlc_data,
        entries,
        interval,
        now,
        start_timestamp,
        data_to_retrieve == 1,
    );

    Ok(())
}

async fn get_entries_from_timestamp(
    pool: &Pool,
    network: Network,
    pair_id: &str,
    start_timestamp: DateTime<Utc>,
) -> Result<Vec<SpotEntry>, InfraError> {
    let raw_sql = format!(
        r#"
            SELECT
                *
            FROM
                {table_name}
            WHERE
                pair_id = '{pair_id}'
                AND timestamp >= '{start_timestamp}'
            ORDER BY
                timestamp
            ASC
        "#,
        table_name = get_table_name(network, DataType::SpotEntry),
        pair_id = pair_id,
        start_timestamp = start_timestamp
    );

    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let entries: Vec<SpotEntry> = conn
        .interact(move |conn| diesel::sql_query(raw_sql).load::<SpotEntry>(conn))
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    Ok(entries)
}

/// Compute the OHLC data from the entries for the given interval.
///
/// The function updates the `ohlc_data` vector with the computed OHLC entries
/// between the `start_timestamp` and the current timestamp.
/// - for the first call, the olhc_data vector is empty and will be populated
///   by multiple OHLC entries - depending on start_timestamp.
/// - for the next calls, the function will update the last OHLC entry in the vector
///   until it closes the current interval. (for example, current timetamp is
///   23h17 and interval is 15mn: we will update this last non finished interval
///   between 23h15 & 23h17).
/// - at some point, current timestamp will close the current interval and
///   we will lock this last interval, for example in our last example 23h15
///   to 23h30, if it's 23h30m03s now, we close the 23h15->23h30 interval
///   and start a new one from 23h30 to 23h30m03s (current time).
fn update_ohlc_data(
    ohlc_data: &mut Vec<OHLCEntry>,
    entries: Vec<SpotEntry>,
    interval: Interval,
    now: DateTime<Utc>,
    mut start_timestamp: DateTime<Utc>,
    only_update_last: bool,
) {
    let interval_duration = Duration::minutes(interval.to_minutes());

    // Remove the last not complete interval to update it.
    // This is because the last entry correspond to the interval
    // closing with current timestamp (so not complete yet).
    if only_update_last {
        ohlc_data.pop();
    }

    while start_timestamp < now {
        let mut end_current_interval = start_timestamp + interval_duration;
        let mut ohlc_end_interval = std::cmp::min(end_current_interval, now);

        let last_ohlc_entry: Option<&OHLCEntry> = ohlc_data.last();

        // If the current time slipped into a new interval, we move
        // start_timestamp to the previous interval - so that we
        // don't miss the last complete interval
        if let Some(last_ohlc_entry) = last_ohlc_entry {
            if only_update_last && (ohlc_end_interval - last_ohlc_entry.time) > interval_duration {
                start_timestamp = last_ohlc_entry.time;
                ohlc_end_interval = interval.align_timestamp(ohlc_end_interval);
                end_current_interval = start_timestamp + interval_duration;
            }
        }

        // get all price entries for the delimited interval
        let entries_for_interval =
            get_entries_for_interval(&entries, start_timestamp, ohlc_end_interval);

        // & compute ohlc from either price entries / last OHLC computed if no entries
        // are available for the current interval
        let maybe_ohlc =
            compute_ohlc_from_entries(&entries_for_interval, ohlc_end_interval, last_ohlc_entry);
        if let Some(ohlc) = maybe_ohlc {
            ohlc_data.push(ohlc);
        }

        // & increase the timestamp for the next interval
        start_timestamp = end_current_interval;
    }
}

fn compute_ohlc_from_entries(
    entries: &[&SpotEntry],
    end_interval: DateTime<Utc>,
    last_ohlc_computed: Option<&OHLCEntry>,
) -> Option<OHLCEntry> {
    if entries.is_empty() && last_ohlc_computed.is_none() {
        return None;
    }

    if !entries.is_empty() {
        // Safe to unwrap since we checked that entries is not empty
        Some(OHLCEntry {
            open: entries.first().unwrap().price.clone(),
            high: entries
                .iter()
                .map(|entry| entry.price.clone())
                .max()
                .unwrap(),
            low: entries
                .iter()
                .map(|entry| entry.price.clone())
                .min()
                .unwrap(),
            close: entries.last().unwrap().price.clone(),
            time: end_interval,
        })
    } else if last_ohlc_computed.is_some() {
        // If no data is available for the current interval and we have
        // a last OHLC computed, we use the last close price as the
        // OHLC values for the current interval.
        let last_ohlc_computed = last_ohlc_computed.unwrap();
        Some(OHLCEntry {
            open: last_ohlc_computed.close.clone(),
            high: last_ohlc_computed.close.clone(),
            low: last_ohlc_computed.close.clone(),
            close: last_ohlc_computed.close.clone(),
            time: end_interval,
        })
    } else {
        None
    }
}

/// Get all entries for a given interval.
/// The interval is defined by the start_timestamp and the end_current_interval.
fn get_entries_for_interval(
    entries: &[SpotEntry],
    start_timestamp: DateTime<Utc>,
    end_current_interval: DateTime<Utc>,
) -> Vec<&SpotEntry> {
    entries
        .iter()
        .filter(|entry| {
            (entry.timestamp >= start_timestamp) && (entry.timestamp <= end_current_interval)
        })
        .collect::<Vec<&SpotEntry>>()
}
