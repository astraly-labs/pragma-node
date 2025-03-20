use std::collections::HashSet;

use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{DateTime, NaiveDateTime};
use diesel::prelude::QueryableByName;
use diesel::sql_types::{Double, Jsonb, Record, VarChar};
use diesel::{Queryable, RunQueryDsl};
use pragma_common::timestamp::TimestampError;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use pragma_common::errors::ConversionError;
use pragma_common::signing::starkex::StarkexPrice;
use pragma_common::types::pair::Pair;
use pragma_common::types::{AggregationMode, DataType, Interval};
use pragma_entities::{Entry, error::InfraError};

use crate::constants::EIGHTEEN_DECIMALS;
use crate::constants::currencies::ABSTRACT_CURRENCIES;
use crate::constants::others::ROUTING_FRESHNESS_THRESHOLD;
use crate::constants::starkex_ws::{
    INITAL_INTERVAL_IN_MS, INTERVAL_INCREMENT_IN_MS, MAX_INTERVAL_WITHOUT_ENTRIES,
    MINIMUM_NUMBER_OF_PUBLISHERS,
};
use crate::handlers::get_entry::EntryParams;
use crate::handlers::subscribe_to_entry::{AssetOraclePrice, SignedPublisherPrice};
use crate::utils::convert_via_quote;
use crate::utils::sql::{get_interval_specifier, get_table_suffix};

#[derive(Debug, Serialize, Queryable)]
pub struct MedianEntry {
    pub time: NaiveDateTime,
    pub median_price: BigDecimal,
    pub num_sources: i64,
    pub components: Option<Vec<Component>>,
}

// Base struct without components
#[derive(Serialize, QueryableByName, Clone, Debug)]
pub struct MedianEntryRawBase {
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub time: NaiveDateTime,

    #[diesel(sql_type = diesel::sql_types::Numeric)]
    pub median_price: BigDecimal,

    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub num_sources: i64,
}

// Extended struct with components (non-optional)
#[derive(Serialize, QueryableByName, Clone, Debug)]
pub struct MedianEntryRawWithComponents {
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub time: NaiveDateTime,

    #[diesel(sql_type = diesel::sql_types::Numeric)]
    pub median_price: BigDecimal,

    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub num_sources: i64,

    #[diesel(sql_type = diesel::sql_types::Array<Record<(diesel::sql_types::Text, diesel::sql_types::Text, diesel::sql_types::Numeric, diesel::sql_types::Timestamptz)>>)]
    pub components: Vec<(String, String, BigDecimal, NaiveDateTime)>,

    #[diesel(sql_type = diesel::sql_types::Bool)]
    pub has_components: bool,
}

#[derive(Serialize, QueryableByName, Clone, Debug)]
pub struct ExpiriesListRaw {
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub expiration_timestamp: NaiveDateTime,
}

pub async fn routing(
    pool: &deadpool_diesel::postgres::Pool,
    is_routing: bool,
    pair: &Pair,
    entry_params: &EntryParams,
    with_components: bool,
) -> Result<MedianEntry, InfraError> {
    // If we have entries for the pair_id and the latest entry is fresh enough,
    // Or if we are not routing, we can return the price directly.
    if !is_routing
        || (pair_id_exist(pool, pair).await?
            && get_last_updated_timestamp(pool, pair.to_pair_id(), entry_params.timestamp)
                .await?
                .unwrap_or(NaiveDateTime::default())
                .and_utc()
                .timestamp()
                >= entry_params.timestamp - ROUTING_FRESHNESS_THRESHOLD)
    {
        return get_price(pool, pair, entry_params, with_components).await;
    }

    (find_alternative_pair_price(pool, pair, entry_params, with_components).await)
        .map_or_else(|_| Err(InfraError::RoutingError(pair.to_pair_id())), Ok)
}

pub fn calculate_rebased_price(
    base_entry: MedianEntry,
    quote_entry: MedianEntry,
) -> Result<MedianEntry, InfraError> {
    if quote_entry.median_price == BigDecimal::from(0) {
        return Err(InfraError::InternalServerError);
    }

    let rebase_price = convert_via_quote(
        base_entry.median_price,
        quote_entry.median_price,
        EIGHTEEN_DECIMALS,
    )?;

    let max_timestamp = std::cmp::max(
        base_entry.time.and_utc().timestamp(),
        quote_entry.time.and_utc().timestamp(),
    );
    let num_sources = std::cmp::max(base_entry.num_sources, quote_entry.num_sources);
    let new_timestamp = DateTime::from_timestamp(max_timestamp, 0)
        .ok_or(InfraError::InvalidTimestamp(
            TimestampError::ToDatetimeErrorI64(max_timestamp),
        ))?
        .naive_utc();

    let median_entry = MedianEntry {
        time: new_timestamp,
        median_price: rebase_price,
        num_sources,
        components: Option::None, // No components for routing
    };

    Ok(median_entry)
}

async fn find_alternative_pair_price(
    pool: &deadpool_diesel::postgres::Pool,
    pair: &Pair,
    entry_params: &EntryParams,
    with_components: bool,
) -> Result<MedianEntry, InfraError> {
    for alt_currency in ABSTRACT_CURRENCIES {
        let base_alt_pair = Pair::from((pair.base.clone(), alt_currency.to_string()));
        let alt_quote_pair = Pair::from((pair.quote.clone(), alt_currency.to_string()));

        if pair_id_exist(pool, &base_alt_pair.clone()).await?
            && pair_id_exist(pool, &alt_quote_pair.clone()).await?
        {
            let base_alt_result =
                get_price(pool, &base_alt_pair, entry_params, with_components).await?;
            let alt_quote_result =
                get_price(pool, &alt_quote_pair, entry_params, with_components).await?;

            return calculate_rebased_price(base_alt_result, alt_quote_result);
        }
    }

    Err(InfraError::RoutingError(pair.to_pair_id()))
}

async fn pair_id_exist(
    pool: &deadpool_diesel::postgres::Pool,
    pair: &Pair,
) -> Result<bool, InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;

    let pair_str = pair.to_string();
    let res = conn
        .interact(move |conn| Entry::exists(conn, pair_str))
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    Ok(res)
}

async fn get_price(
    pool: &deadpool_diesel::postgres::Pool,
    pair: &Pair,
    entry_params: &EntryParams,
    with_components: bool,
) -> Result<MedianEntry, InfraError> {
    let entry = match entry_params.aggregation_mode {
        AggregationMode::Median => {
            get_median_price(pool, pair.to_pair_id(), entry_params, with_components).await?
        }
        AggregationMode::Twap => {
            get_twap_price(pool, pair.to_pair_id(), entry_params, with_components).await?
        }
    };

    Ok(entry)
}

pub async fn get_twap_price(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
    entry_params: &EntryParams,
    with_components: bool,
) -> Result<MedianEntry, InfraError> {
    if with_components {
        get_twap_price_with_components(pool, pair_id, entry_params).await
    } else {
        get_twap_price_without_components(pool, pair_id, entry_params).await
    }
}

// Function to get TWAP price without components
pub async fn get_twap_price_without_components(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
    entry_params: &EntryParams,
) -> Result<MedianEntry, InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;

    let sql_request: String = format!(
        r"
        SELECT
            bucket AS time,
            price_twap AS median_price,
            num_sources
        FROM
            twap_{}_agg{}
        WHERE
            pair_id = $1
            AND
            bucket <= $2
        ORDER BY
            time DESC
        LIMIT 1;
    ",
        get_interval_specifier(entry_params.interval, true)?,
        get_table_suffix(entry_params.data_type)?,
    );

    let date_time = DateTime::from_timestamp(entry_params.timestamp, 0).ok_or(
        InfraError::InvalidTimestamp(TimestampError::ToDatetimeErrorI64(entry_params.timestamp)),
    )?;

    let p = pair_id.clone();
    let raw_entry = conn
        .interact(move |conn| {
            diesel::sql_query(&sql_request)
                .bind::<diesel::sql_types::Text, _>(p)
                .bind::<diesel::sql_types::Timestamptz, _>(date_time)
                .load::<MedianEntryRawBase>(conn)
        })
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    let raw_entry = raw_entry
        .first()
        .ok_or(InfraError::EntryNotFound(pair_id))?;

    Ok(MedianEntry {
        time: raw_entry.time,
        median_price: raw_entry.median_price.clone(),
        num_sources: raw_entry.num_sources,
        components: None,
    })
}

// Function to get TWAP price with components - optimized to avoid extra check
pub async fn get_twap_price_with_components(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
    entry_params: &EntryParams,
) -> Result<MedianEntry, InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;

    // Get components and check existence in a single query
    let sql_request: String = format!(
        r"
        SELECT
            bucket AS time,
            price_twap AS median_price,
            num_sources,
            COALESCE(components, ARRAY[]::record(text,text,numeric,timestamptz)[]) as components,
            (components IS NOT NULL AND array_length(components, 1) > 0) as has_components
        FROM
            twap_{}_agg{}
        WHERE
            pair_id = $1
            AND
            bucket <= $2
        ORDER BY
            time DESC
        LIMIT 1;
        ",
        get_interval_specifier(entry_params.interval, true)?,
        get_table_suffix(entry_params.data_type)?,
    );

    let date_time = DateTime::from_timestamp(entry_params.timestamp, 0).ok_or(
        InfraError::InvalidTimestamp(TimestampError::ToDatetimeErrorI64(entry_params.timestamp)),
    )?;

    let p = pair_id.clone();
    let raw_entry = conn
        .interact(move |conn| {
            diesel::sql_query(&sql_request)
                .bind::<diesel::sql_types::Text, _>(p)
                .bind::<diesel::sql_types::Timestamptz, _>(date_time)
                .load::<MedianEntryRawWithComponents>(conn)
        })
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    let raw_entry = raw_entry
        .first()
        .ok_or(InfraError::EntryNotFound(pair_id))?;

    // Only create components if the flag indicates they exist
    let components = if raw_entry.has_components {
        Some(
            raw_entry
                .components
                .iter()
                .map(|(source, publisher, price, timestamp)| Component {
                    source: source.clone(),
                    publisher: publisher.clone(),
                    price: price.clone(),
                    timestamp: *timestamp,
                })
                .collect(),
        )
    } else {
        None
    };

    Ok(MedianEntry {
        time: raw_entry.time,
        median_price: raw_entry.median_price.clone(),
        num_sources: raw_entry.num_sources,
        components,
    })
}

// Function to get median price without components
pub async fn get_median_price_without_components(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
    entry_params: &EntryParams,
) -> Result<MedianEntry, InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;

    let sql_request: String = format!(
        r"
        SELECT
            bucket AS time,
            median_price,
            num_sources
        FROM
            price_{}_agg{}
        WHERE
            pair_id = $1
            AND
            bucket <= $2
        ORDER BY
            time DESC
        LIMIT 1;
    ",
        get_interval_specifier(entry_params.interval, false)?,
        get_table_suffix(entry_params.data_type)?,
    );

    let date_time = DateTime::from_timestamp(entry_params.timestamp, 0).ok_or(
        InfraError::InvalidTimestamp(TimestampError::ToDatetimeErrorI64(entry_params.timestamp)),
    )?;

    let p = pair_id.clone();
    let raw_entry = conn
        .interact(move |conn| {
            diesel::sql_query(&sql_request)
                .bind::<diesel::sql_types::Text, _>(p)
                .bind::<diesel::sql_types::Timestamptz, _>(date_time)
                .load::<MedianEntryRawBase>(conn)
        })
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    let raw_entry = raw_entry
        .first()
        .ok_or(InfraError::EntryNotFound(pair_id))?;

    Ok(MedianEntry {
        time: raw_entry.time,
        median_price: raw_entry.median_price.clone(),
        num_sources: raw_entry.num_sources,
        components: None,
    })
}

// Function to get median price with components - optimized to avoid extra check
pub async fn get_median_price_with_components(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
    entry_params: &EntryParams,
) -> Result<MedianEntry, InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;

    // Get components and check existence in a single query
    let sql_request: String = format!(
        r"
        SELECT
            bucket AS time,
            median_price,
            num_sources,
            COALESCE(components, ARRAY[]::record(text,text,numeric,timestamptz)[]) as components,
            (components IS NOT NULL AND array_length(components, 1) > 0) as has_components
        FROM
            price_{}_agg{}
        WHERE
            pair_id = $1
            AND
            bucket <= $2
        ORDER BY
            time DESC
        LIMIT 1;
        ",
        get_interval_specifier(entry_params.interval, false)?,
        get_table_suffix(entry_params.data_type)?,
    );

    let date_time = DateTime::from_timestamp(entry_params.timestamp, 0).ok_or(
        InfraError::InvalidTimestamp(TimestampError::ToDatetimeErrorI64(entry_params.timestamp)),
    )?;

    let p = pair_id.clone();
    let raw_entry = conn
        .interact(move |conn| {
            diesel::sql_query(&sql_request)
                .bind::<diesel::sql_types::Text, _>(p)
                .bind::<diesel::sql_types::Timestamptz, _>(date_time)
                .load::<MedianEntryRawWithComponents>(conn)
        })
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    let raw_entry = raw_entry
        .first()
        .ok_or(InfraError::EntryNotFound(pair_id))?;

    // Only create components if the flag indicates they exist
    let components = if raw_entry.has_components {
        Some(
            raw_entry
                .components
                .iter()
                .map(|(source, publisher, price, timestamp)| Component {
                    source: source.clone(),
                    publisher: publisher.clone(),
                    price: price.clone(),
                    timestamp: *timestamp,
                })
                .collect(),
        )
    } else {
        None
    };

    Ok(MedianEntry {
        time: raw_entry.time,
        median_price: raw_entry.median_price.clone(),
        num_sources: raw_entry.num_sources,
        components,
    })
}

// Wrapper function for backward compatibility
pub async fn get_median_price(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
    entry_params: &EntryParams,
    include_components: bool,
) -> Result<MedianEntry, InfraError> {
    if include_components {
        get_median_price_with_components(pool, pair_id, entry_params).await
    } else {
        get_median_price_without_components(pool, pair_id, entry_params).await
    }
}

pub async fn get_median_entries_1_min_between(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
    start_timestamp: u64,
    end_timestamp: u64,
) -> Result<Vec<MedianEntry>, InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;
    #[allow(clippy::cast_possible_wrap)]
    let start_datetime = DateTime::from_timestamp(start_timestamp as i64, 0).ok_or(
        InfraError::InvalidTimestamp(TimestampError::ToDatetimeErrorU64(start_timestamp)),
    )?;
    #[allow(clippy::cast_possible_wrap)]
    let end_datetime = DateTime::from_timestamp(end_timestamp as i64, 0).ok_or(
        InfraError::InvalidTimestamp(TimestampError::ToDatetimeErrorU64(start_timestamp)),
    )?;

    let raw_sql = r"
        SELECT
            bucket AS time,
            median_price,
            num_sources,
            NULL as components
        FROM price_1_min_agg
        WHERE 
            pair_id = $1
        AND 
            time BETWEEN $2 AND $3
        ORDER BY 
            time DESC;
    ";

    let raw_entries = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql)
                .bind::<diesel::sql_types::Text, _>(pair_id)
                .bind::<diesel::sql_types::Timestamptz, _>(start_datetime)
                .bind::<diesel::sql_types::Timestamptz, _>(end_datetime)
                .load::<MedianEntryRawWithComponents>(conn)
        })
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    let entries: Vec<MedianEntry> = raw_entries
        .into_iter()
        .map(|raw_entry| MedianEntry {
            time: raw_entry.time,
            median_price: raw_entry.median_price,
            num_sources: raw_entry.num_sources,
            components: Option::None,
        })
        .collect();

    Ok(entries)
}

pub async fn get_median_prices_between(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
    entry_params: EntryParams,
    start_timestamp: u64,
    end_timestamp: u64,
) -> Result<Vec<MedianEntry>, InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;
    #[allow(clippy::cast_possible_wrap)]
    let start_datetime = DateTime::from_timestamp(start_timestamp as i64, 0).ok_or(
        InfraError::InvalidTimestamp(TimestampError::ToDatetimeErrorU64(start_timestamp)),
    )?;
    #[allow(clippy::cast_possible_wrap)]
    let end_datetime = DateTime::from_timestamp(end_timestamp as i64, 0).ok_or(
        InfraError::InvalidTimestamp(TimestampError::ToDatetimeErrorU64(end_timestamp)),
    )?;

    let sql_request: String = format!(
        r"
        -- query the materialized realtime view
        SELECT
            bucket AS time,
            median_price,
            num_sources, 
            COALESCE(components, ARRAY[]::record(text,text,numeric,timestamptz)[]) as components,
            (components IS NOT NULL AND array_length(components, 1) > 0) as has_components
        FROM
            price_{}_agg{}
        WHERE
            pair_id = $1
            AND
            bucket BETWEEN $2 AND $3
        ORDER BY
            time DESC;
    ",
        get_interval_specifier(entry_params.interval, false)?,
        get_table_suffix(entry_params.data_type)?,
    );

    let raw_entries = conn
        .interact(move |conn| {
            diesel::sql_query(&sql_request)
                .bind::<diesel::sql_types::Text, _>(pair_id)
                .bind::<diesel::sql_types::Timestamptz, _>(start_datetime)
                .bind::<diesel::sql_types::Timestamptz, _>(end_datetime)
                .load::<MedianEntryRawWithComponents>(conn)
        })
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    let entries: Vec<MedianEntry> = raw_entries
        .into_iter()
        .map(|raw_entry| {
            // Process components only if they exist
            let components = if raw_entry.has_components {
                Some(
                    raw_entry
                        .components
                        .iter()
                        .map(|(source, publisher, price, timestamp)| Component {
                            source: source.clone(),
                            publisher: publisher.clone(),
                            price: price.clone(),
                            timestamp: *timestamp,
                        })
                        .collect(),
                )
            } else {
                None
            };

            MedianEntry {
                time: raw_entry.time,
                median_price: raw_entry.median_price,
                num_sources: raw_entry.num_sources,
                components,
            }
        })
        .collect();

    Ok(entries)
}

pub async fn get_twap_prices_between(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
    entry_params: EntryParams,
    start_timestamp: u64,
    end_timestamp: u64,
) -> Result<Vec<MedianEntry>, InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;
    #[allow(clippy::cast_possible_wrap)]
    let start_datetime = DateTime::from_timestamp(start_timestamp as i64, 0).ok_or(
        InfraError::InvalidTimestamp(TimestampError::ToDatetimeErrorU64(start_timestamp)),
    )?;
    #[allow(clippy::cast_possible_wrap)]
    let end_datetime = DateTime::from_timestamp(end_timestamp as i64, 0).ok_or(
        InfraError::InvalidTimestamp(TimestampError::ToDatetimeErrorU64(end_timestamp)),
    )?;

    let sql_request: String = format!(
        r"
        -- query the materialized realtime view
        SELECT
            bucket AS time,
            price_twap AS median_price,
            num_sources,
            NULL as components
        FROM
            twap_{}_agg{}
        WHERE
            pair_id = $1
            AND
            bucket BETWEEN $2 AND $3
        ORDER BY
            time DESC;
    ",
        get_interval_specifier(entry_params.interval, true)?,
        get_table_suffix(entry_params.data_type)?,
    );

    let raw_entries = conn
        .interact(move |conn| {
            diesel::sql_query(&sql_request)
                .bind::<diesel::sql_types::Text, _>(pair_id)
                .bind::<diesel::sql_types::Timestamptz, _>(start_datetime)
                .bind::<diesel::sql_types::Timestamptz, _>(end_datetime)
                .load::<MedianEntryRawWithComponents>(conn)
        })
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    let entries: Vec<MedianEntry> = raw_entries
        .into_iter()
        .map(|raw_entry| MedianEntry {
            time: raw_entry.time,
            median_price: raw_entry.median_price,
            num_sources: raw_entry.num_sources,
            components: Option::None,
        })
        .collect();

    Ok(entries)
}

pub async fn get_last_updated_timestamp(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
    max_timestamp: i64,
) -> Result<Option<NaiveDateTime>, InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;
    conn.interact(move |conn| Entry::get_last_updated_timestamp(conn, pair_id, max_timestamp))
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, ToSchema)]
pub struct OHLCEntry {
    pub time: NaiveDateTime,
    #[schema(value_type = u64)]
    pub open: BigDecimal,
    #[schema(value_type = u64)]
    pub low: BigDecimal,
    #[schema(value_type = u64)]
    pub high: BigDecimal,
    #[schema(value_type = u64)]
    pub close: BigDecimal,
}

#[derive(Serialize, QueryableByName, Clone, Debug)]
pub struct OHLCEntryRaw {
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub time: NaiveDateTime,
    #[diesel(sql_type = diesel::sql_types::Numeric)]
    pub open: BigDecimal,
    #[diesel(sql_type = diesel::sql_types::Numeric)]
    pub high: BigDecimal,
    #[diesel(sql_type = diesel::sql_types::Numeric)]
    pub low: BigDecimal,
    #[diesel(sql_type = diesel::sql_types::Numeric)]
    pub close: BigDecimal,
}

impl From<OHLCEntryRaw> for OHLCEntry {
    fn from(raw: OHLCEntryRaw) -> Self {
        Self {
            time: raw.time,
            open: raw.open,
            high: raw.high,
            low: raw.low,
            close: raw.close,
        }
    }
}

impl FromIterator<OHLCEntryRaw> for Vec<OHLCEntry> {
    fn from_iter<T: IntoIterator<Item = OHLCEntryRaw>>(iter: T) -> Self {
        iter.into_iter().map(OHLCEntry::from).collect()
    }
}

pub async fn get_ohlc(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
    interval: Interval,
    time: i64,
) -> Result<Vec<OHLCEntry>, InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;

    let raw_sql = format!(
        r"
        -- query the materialized realtime view
        SELECT
            ohlc_bucket AS time,
            open,
            high,
            low,
            close
        FROM
            candle_{}
        WHERE
            pair_id = $1
            AND
            ohlc_bucket <= $2
        ORDER BY
            time DESC
        LIMIT 10000;
    ",
        get_interval_specifier(interval, false)?
    );

    let date_time = DateTime::from_timestamp(time, 0).ok_or(InfraError::InvalidTimestamp(
        TimestampError::ToDatetimeErrorI64(time),
    ))?;

    let raw_entries = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql)
                .bind::<diesel::sql_types::Text, _>(pair_id)
                .bind::<diesel::sql_types::Timestamptz, _>(date_time)
                .load::<OHLCEntryRaw>(conn)
        })
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

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

#[derive(Debug, Queryable, QueryableByName, Deserialize, Serialize)]
struct RawMedianEntryWithComponents {
    #[diesel(sql_type = VarChar)]
    pub pair_id: String,
    #[diesel(sql_type = Double)]
    pub median_price: f64,
    #[diesel(sql_type = Jsonb)]
    pub components: serde_json::Value,
}

impl TryFrom<RawMedianEntryWithComponents> for MedianEntryWithComponents {
    type Error = ConversionError;

    fn try_from(raw: RawMedianEntryWithComponents) -> Result<Self, Self::Error> {
        let components: Vec<EntryComponent> =
            serde_json::from_value(raw.components).map_err(|_| Self::Error::FailedSerialization)?;

        // The database returns us the timestamp in RFC3339 format, so we
        // need to convert it to a Unix timestamp before going further.
        let components = components
            .into_iter()
            .map(|c| {
                Ok(EntryComponent {
                    timestamp: DateTime::parse_from_rfc3339(&c.timestamp)
                        .map_err(|_| Self::Error::InvalidDateTime)?
                        .timestamp()
                        .to_string(),
                    ..c
                })
            })
            .collect::<Result<Vec<EntryComponent>, Self::Error>>()?;

        let median_price =
            BigDecimal::from_f64(raw.median_price).ok_or(Self::Error::BigDecimalConversion)?;

        Ok(Self {
            pair_id: raw.pair_id,
            median_price,
            components,
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EntryComponent {
    pub pair_id: String,
    pub price: BigDecimal,
    pub timestamp: String,
    pub publisher: String,
    pub publisher_address: String,
    pub publisher_signature: String,
}

impl TryFrom<EntryComponent> for SignedPublisherPrice {
    type Error = ConversionError;

    fn try_from(component: EntryComponent) -> Result<Self, Self::Error> {
        let asset_id = StarkexPrice::get_oracle_asset_id(&component.publisher, &component.pair_id)?;

        Ok(Self {
            oracle_asset_id: format!("0x{asset_id}"),
            oracle_price: component.price.to_string(),
            timestamp: component.timestamp.to_string(),
            signing_key: component.publisher_address,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MedianEntryWithComponents {
    pub pair_id: String,
    pub median_price: BigDecimal,
    pub components: Vec<EntryComponent>,
}

impl TryFrom<MedianEntryWithComponents> for AssetOraclePrice {
    type Error = ConversionError;

    fn try_from(median_entry: MedianEntryWithComponents) -> Result<Self, Self::Error> {
        let signed_prices: Result<Vec<SignedPublisherPrice>, ConversionError> = median_entry
            .components
            .into_iter()
            .map(SignedPublisherPrice::try_from)
            .collect();

        let global_asset_id = StarkexPrice::get_global_asset_id(&median_entry.pair_id)?;

        Ok(Self {
            global_asset_id: format!("0x{global_asset_id}"),
            median_price: median_entry.median_price.to_string(),
            signed_prices: signed_prices?,
            signature: Default::default(),
        })
    }
}

/// Convert a list of raw entries into a list of valid median entries.
/// For each `pair_id`, check if it has a valid median price with enough unique publishers.
/// Returns the valid entries, filtering out any invalid ones.
fn get_median_entries_response(
    raw_entries: Vec<RawMedianEntryWithComponents>,
) -> Option<Vec<MedianEntryWithComponents>> {
    if raw_entries.is_empty() {
        return None;
    }

    let mut valid_entries = Vec::new();

    for raw_entry in raw_entries {
        let pair_id = raw_entry.pair_id.clone();
        let median_entry = match MedianEntryWithComponents::try_from(raw_entry) {
            Ok(entry) => entry,
            Err(e) => {
                tracing::error!(
                    "Cannot convert raw median entry to median entry for pair {}: {:?}",
                    pair_id,
                    e
                );
                continue;
            }
        };

        let num_unique_publishers = median_entry
            .components
            .iter()
            .map(|c| &c.publisher)
            .collect::<HashSet<_>>()
            .len();

        if num_unique_publishers >= MINIMUM_NUMBER_OF_PUBLISHERS {
            valid_entries.push(median_entry);
        } else {
            tracing::warn!(
                "Insufficient unique publishers for pair {}: got {}, need {}",
                median_entry.pair_id,
                num_unique_publishers,
                MINIMUM_NUMBER_OF_PUBLISHERS
            );
        }
    }

    (!valid_entries.is_empty()).then_some(valid_entries)
}

/// Retrieves the timescale table name for the given entry type.
const fn get_table_name_from_type(entry_type: DataType) -> &'static str {
    match entry_type {
        DataType::SpotEntry => "entries",
        DataType::FutureEntry | DataType::PerpEntry => "future_entries",
    }
}

/// We exclude PRAGMA publisher for starkex endpoint as data is not reliable for that use case.
/// One solution would be to adapt the price-pusher to push prices as soon as they are available.
/// For now, we prefer to just work with data from 1st party sources.
const EXCLUDED_PUBLISHER: &str = "";

/// Builds a SQL query that will fetch the recent prices between now and
/// the given interval for each unique tuple (`pair_id`, publisher, source)
/// and then calculate the median price for each `pair_id`.
/// We also return in a JSON string the components that were used to calculate
/// the median price.
fn build_sql_query_for_median_with_components(
    pair_ids: &[String],
    interval_in_ms: u64,
    entry_type: DataType,
) -> String {
    format!(
        r"
            WITH last_prices AS (
                SELECT
                    e.pair_id,
                    e.publisher,
                    p.account_address AS publisher_account_address,
                    e.source,
                    e.price,
                    e.timestamp,
                    e.publisher_signature,
                    ROW_NUMBER() OVER (PARTITION BY e.pair_id, e.publisher, e.source ORDER BY e.timestamp DESC) AS rn
                FROM 
                    {table_name} e
                JOIN
                    publishers p ON e.publisher = p.name
                WHERE 
                    e.pair_id IN ({pairs_list})
                    AND e.timestamp >= NOW() - INTERVAL '{interval_in_ms} milliseconds'
                    AND e.publisher != '{excluded_publisher}'
                    {perp_filter}
            ),
            filtered_last_prices AS (
                SELECT 
                    pair_id,
                    publisher,
                    publisher_account_address,
                    source,
                    price,
                    timestamp,
                    publisher_signature
                FROM 
                    last_prices
                WHERE 
                    rn = 1
            )
            SELECT
                pair_id,
                percentile_cont(0.5) WITHIN GROUP (ORDER BY price) AS median_price,
                jsonb_agg(
			        jsonb_build_object(
			            'pair_id', pair_id,
			            'price', price,
			            'timestamp', timestamp,
			            'publisher', publisher,
                        'publisher_address', publisher_account_address,
			            'publisher_signature', publisher_signature
			        )
			    ) AS components
            FROM
                filtered_last_prices
            GROUP BY 
                pair_id;
            ",
        table_name = get_table_name_from_type(entry_type),
        pairs_list = pair_ids
            .iter()
            .map(|pair_id| format!("'{pair_id}'"))
            .collect::<Vec<String>>()
            .join(", "),
        interval_in_ms = interval_in_ms,
        excluded_publisher = EXCLUDED_PUBLISHER,
        perp_filter = match entry_type {
            DataType::PerpEntry => "AND e.expiration_timestamp IS NULL",
            _ => "",
        }
    )
}

/// Compute the median price for each `pair_id` in the given list of `pair_ids`
/// over an interval of time.
///
/// The interval is increased until we have valid entries with enough publishers.
/// Returns any pairs that have valid data, even if some pairs are invalid.
pub async fn get_current_median_entries_with_components(
    pool: &deadpool_diesel::postgres::Pool,
    pair_ids: &[String],
    entry_type: DataType,
) -> Result<Vec<MedianEntryWithComponents>, InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;
    let mut interval_in_ms = INITAL_INTERVAL_IN_MS;
    let mut last_valid_entries = Vec::new();

    loop {
        let raw_sql =
            build_sql_query_for_median_with_components(pair_ids, interval_in_ms, entry_type);

        let raw_median_entries = conn
            .interact(move |conn| {
                diesel::sql_query(raw_sql).load::<RawMedianEntryWithComponents>(conn)
            })
            .await
            .map_err(InfraError::DbInteractionError)?
            .map_err(InfraError::DbResultError)?;

        if let Some(valid_entries) = get_median_entries_response(raw_median_entries) {
            // Keep track of the valid entries we've found
            last_valid_entries = valid_entries;

            // If we have valid entries for all pairs, we can return early
            let found_pairs: HashSet<_> = last_valid_entries.iter().map(|e| &e.pair_id).collect();
            let requested_pairs: HashSet<_> = pair_ids.iter().collect();
            if found_pairs == requested_pairs {
                break;
            }
        }

        interval_in_ms += INTERVAL_INCREMENT_IN_MS;

        if interval_in_ms >= MAX_INTERVAL_WITHOUT_ENTRIES {
            // Log which pairs we couldn't get valid data for
            let found_pairs: HashSet<_> = last_valid_entries
                .iter()
                .map(|e| e.pair_id.clone())
                .collect();
            let missing_pairs: Vec<_> = pair_ids
                .iter()
                .filter(|p| !found_pairs.contains(*p))
                .collect();

            if !missing_pairs.is_empty() {
                tracing::warn!(
                    "Could not compute valid median entries for pairs: {}, [{:?}]",
                    missing_pairs
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    entry_type
                );
            }
            break;
        }
    }

    Ok(last_valid_entries)
}

pub async fn get_expiries_list(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
) -> Result<Vec<NaiveDateTime>, InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;

    let sql_request: String = r"
        SELECT DISTINCT expiration_timestamp
        FROM future_entries
        WHERE pair_id = $1 AND expiration_timestamp IS NOT NULL
        ORDER BY expiration_timestamp;
        "
    .to_string();

    let raw_exp = conn
        .interact(move |conn| {
            diesel::sql_query(&sql_request)
                .bind::<diesel::sql_types::Text, _>(pair_id)
                .load::<ExpiriesListRaw>(conn)
        })
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    let expiries: Vec<NaiveDateTime> = raw_exp
        .into_iter()
        .map(|r| r.expiration_timestamp)
        .collect();

    Ok(expiries)
}

// Add a new struct to hold the individual price data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Component {
    pub source: String,
    pub publisher: String,
    pub price: BigDecimal,
    pub timestamp: NaiveDateTime,
}

// Add implementation of From for converting between Component and EntryComponent
impl From<Component> for crate::handlers::get_entry::EntryComponent {
    fn from(individual: Component) -> Self {
        Self {
            source: individual.source,
            publisher: individual.publisher,
            price: individual.price.to_string(),
            timestamp: individual.timestamp.and_utc().timestamp_millis() as u64,
        }
    }
}

// Add reverse conversion
impl TryFrom<crate::handlers::get_entry::EntryComponent> for Component {
    type Error = InfraError;

    fn try_from(
        component: crate::handlers::get_entry::EntryComponent,
    ) -> Result<Self, Self::Error> {
        let price = component
            .price
            .parse::<BigDecimal>()
            .map_err(|_| InfraError::InternalServerError)?;

        let timestamp = DateTime::from_timestamp_millis(component.timestamp as i64)
            .ok_or(InfraError::InvalidTimestamp(
                TimestampError::ToDatetimeErrorI64(component.timestamp as i64),
            ))?
            .naive_utc();

        Ok(Self {
            source: component.source,
            publisher: component.publisher,
            price,
            timestamp,
        })
    }
}
