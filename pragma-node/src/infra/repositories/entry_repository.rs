use std::collections::HashSet;

use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use chrono::{DateTime, NaiveDateTime};
use diesel::prelude::QueryableByName;
use diesel::sql_types::Record;
use diesel::{Queryable, RunQueryDsl};
use pragma_common::signing::{Signable, sign_data};
use pragma_common::timestamp::TimestampError;
use pragma_entities::EntryError;
use serde::{Deserialize, Serialize};
use starknet::core::utils::cairo_short_string_to_felt;
use starknet::signers::SigningKey;
use starknet_crypto::{Felt, pedersen_hash};
use utoipa::ToSchema;

use pragma_common::errors::ConversionError;
use pragma_common::signing::starkex::StarkexPrice;
use pragma_common::timestamp::TimestampError;
use pragma_common::types::pair::Pair;
use pragma_common::types::{AggregationMode, Interval};
use pragma_entities::{Entry, error::InfraError};

use crate::constants::EIGHTEEN_DECIMALS;
use crate::constants::currencies::ABSTRACT_CURRENCIES;
use crate::constants::others::ROUTING_FRESHNESS_THRESHOLD;
use crate::constants::starkex_ws::{
    INITAL_INTERVAL_IN_MS, INTERVAL_INCREMENT_IN_MS, MAX_INTERVAL_WITHOUT_ENTRIES,
    MINIMUM_NUMBER_OF_PUBLISHERS, PRAGMA_ORACLE_NAME_FOR_STARKEX,
};
use crate::handlers::get_entry::EntryParams;
use crate::handlers::subscribe_to_entry::{AssetOraclePrice, SignedPublisherPrice};
use crate::utils::convert_via_quote;
use crate::utils::sql::{get_interval_specifier, get_table_suffix};

use super::utils::HexFormat;

#[derive(Debug, Serialize, Queryable)]
pub struct MedianEntry {
    pub time: NaiveDateTime,
    pub median_price: BigDecimal,
    pub num_sources: i64,
    pub components: Option<Vec<Component>>,
}

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

    #[diesel(sql_type = diesel::sql_types::Array<Record<(diesel::sql_types::Text, diesel::sql_types::Numeric, diesel::sql_types::Timestamptz)>>)]
    pub components: Vec<(String, BigDecimal, NaiveDateTime)>,
}

// Extended struct with components (non-optional)
#[derive(Serialize, QueryableByName, Clone, Debug)]
pub struct ExtendedMedianEntryRaw {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub pair_id: String,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub time: NaiveDateTime,

    #[diesel(sql_type = diesel::sql_types::Numeric)]
    pub median_price: BigDecimal,

    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub num_sources: i64,

    #[diesel(sql_type = diesel::sql_types::Array<Record<(diesel::sql_types::Text, diesel::sql_types::Numeric, diesel::sql_types::Timestamptz)>>)]
    pub components: Vec<(String, BigDecimal, NaiveDateTime)>,
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
        return get_price(pool, pair, entry_params).await;
    }

    (find_alternative_pair_price(pool, pair, entry_params).await)
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
        components: None,
    };

    Ok(median_entry)
}

async fn find_alternative_pair_price(
    pool: &deadpool_diesel::postgres::Pool,
    pair: &Pair,
    entry_params: &EntryParams,
) -> Result<MedianEntry, InfraError> {
    for alt_currency in ABSTRACT_CURRENCIES {
        let base_alt_pair = Pair::from((pair.base.clone(), alt_currency.to_string()));
        let alt_quote_pair = Pair::from((pair.quote.clone(), alt_currency.to_string()));

        if pair_id_exist(pool, &base_alt_pair.clone()).await?
            && pair_id_exist(pool, &alt_quote_pair.clone()).await?
        {
            let base_alt_result = get_price(pool, &base_alt_pair, entry_params).await?;
            let alt_quote_result = get_price(pool, &alt_quote_pair, entry_params).await?;

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
) -> Result<MedianEntry, InfraError> {
    let entry = match entry_params.aggregation_mode {
        AggregationMode::Median => get_median_price(pool, pair.to_pair_id(), entry_params).await?,
        AggregationMode::Twap => get_twap_price(pool, pair.to_pair_id(), entry_params).await?,
    };

    Ok(entry)
}

pub async fn get_twap_price(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
    entry_params: &EntryParams,
) -> Result<MedianEntry, InfraError> {
    if entry_params.with_components {
        get_twap_price_with_components(pool, pair_id, entry_params).await
    } else {
        get_twap_price_without_components(pool, pair_id, entry_params).await
    }
}

pub async fn get_twap_price_without_components(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
    entry_params: &EntryParams,
) -> Result<MedianEntry, InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;

    let sql_request: String = format!(
        r"
        -- query the materialized realtime view
        SELECT
            bucket AS time,
            twap_price AS median_price,
            num_sources
        FROM
            twap_{}_{}
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

    let entry: MedianEntry = MedianEntry {
        time: raw_entry.time,
        median_price: raw_entry.median_price.clone(),
        num_sources: raw_entry.num_sources,
        components: None,
    };

    Ok(entry)
}

// Function to get TWAP price with components
pub async fn get_twap_price_with_components(
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
            num_sources,
            components
        FROM
            twap_{}_{}
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

    // Convert components if they exist
    let components = (!raw_entry.components.is_empty()).then(|| {
        raw_entry
            .components
            .iter()
            .map(ComponentConverter::to_component)
            .collect()
    });

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
) -> Result<MedianEntry, InfraError> {
    if entry_params.with_components {
        get_median_price_with_components(pool, pair_id, entry_params).await
    } else {
        get_median_price_without_components(pool, pair_id, entry_params).await
    }
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
        -- query the materialized realtime view
        SELECT
            bucket AS time,
            median_price,
            num_sources
        FROM
            median_{}_{}
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

    let entry: MedianEntry = MedianEntry {
        time: raw_entry.time,
        median_price: raw_entry.median_price.clone(),
        num_sources: raw_entry.num_sources,
        components: None,
    };

    Ok(entry)
}

// Function to get median price with components
pub async fn get_median_price_with_components(
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
            num_sources,
            components
        FROM
            median_{}_{}
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

    // Convert components if they exist
    let components = (!raw_entry.components.is_empty()).then(|| {
        raw_entry
            .components
            .iter()
            .map(ComponentConverter::to_component)
            .collect()
    });

    Ok(MedianEntry {
        time: raw_entry.time,
        median_price: raw_entry.median_price.clone(),
        num_sources: raw_entry.num_sources,
        components,
    })
}

pub async fn get_spot_median_entries_1_min_between(
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
            num_sources
        FROM median_1_min_spot
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
                .load::<MedianEntryRawBase>(conn)
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
            components: None,
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
            components
        FROM
            median_{}_{}
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
            let components = (!raw_entry.components.is_empty()).then(|| {
                raw_entry
                    .components
                    .iter()
                    .map(ComponentConverter::to_component)
                    .collect()
            });

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
            components
        FROM
            twap_{}_{}
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
        .map(|raw_entry| {
            let components = (!raw_entry.components.is_empty()).then(|| {
                raw_entry
                    .components
                    .iter()
                    .map(ComponentConverter::to_component)
                    .collect()
            });
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

// struct to hold the individual price data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Component {
    pub source: String,
    pub price: String,
    pub timestamp: NaiveDateTime,
}

impl From<Component> for crate::handlers::get_entry::EntryComponent {
    fn from(individual: Component) -> Self {
        Self {
            source: individual.source,
            price: individual.price,
            timestamp: individual.timestamp.and_utc().timestamp_millis() as u64,
        }
    }
}

// Reverse conversion
impl TryFrom<crate::handlers::get_entry::EntryComponent> for Component {
    type Error = InfraError;

    fn try_from(
        component: crate::handlers::get_entry::EntryComponent,
    ) -> Result<Self, Self::Error> {
        let price = component
            .price
            .parse::<BigDecimal>()
            .map_err(|_| InfraError::InternalServerError)?;
        #[allow(clippy::cast_possible_wrap)]
        let timestamp = DateTime::from_timestamp_millis(component.timestamp as i64)
            .ok_or(InfraError::InvalidTimestamp(
                #[allow(clippy::cast_possible_wrap)]
                TimestampError::ToDatetimeErrorI64(component.timestamp as i64),
            ))?
            .naive_utc();

        Ok(Self {
            source: component.source,
            price: price.to_hex_string(),
            timestamp,
        })
    }
}

trait ComponentConverter {
    fn to_component(&self) -> Component;
}

impl ComponentConverter for (String, BigDecimal, NaiveDateTime) {
    fn to_component(&self) -> Component {
        Component {
            source: self.0.clone(),
            price: self.1.to_hex_string(),
            timestamp: self.2,
        }
    }
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

pub async fn get_spot_ohlc(
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
            candle_{}_spot
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

impl SignedPublisherPrice {
    fn from_entry_component(
        component: EntryComponent,
        pragma_signer: &SigningKey,
    ) -> Result<Self, EntryError> {
        let asset_id = StarkexPrice::get_oracle_asset_id(&component.publisher, &component.pair_id)
            .map_err(|e| EntryError::InternalServerError(format!("Conversion error: {:?}", e)))?;
        let signature =
            sign_data(pragma_signer, &component).map_err(|_| EntryError::InvalidSigner)?;

        Ok(Self {
            oracle_asset_id: format!("0x{asset_id}"),
            oracle_price: component.price.to_string(),
            timestamp: component.timestamp.to_string(),
            signing_key: component.publisher_address,
            signature,
        })
    }
}

impl EntryComponent {
    pub fn get_global_asset_id(pair_id: &str) -> Result<String, ConversionError> {
        let pair_id = pair_id.replace('/', "-");
        let pair_id = if pair_id.contains('-') {
            format!("{pair_id}-8")
        } else {
            let (first, second) = pair_id.split_at(3);
            format!("{first}-{second}-8")
        };

        let felt =
            cairo_short_string_to_felt(&pair_id).map_err(|_| ConversionError::FeltConversion)?;
        let hex = format!("{felt:x}");
        Ok(format!("{hex:0<30}"))
    }

    pub fn get_oracle_asset_id(
        oracle_name: &str,
        pair_id: &str,
    ) -> Result<String, ConversionError> {
        let market_name = pair_id.replace(['/', '-'], "");

        let market_felt = cairo_short_string_to_felt(&market_name)
            .map_err(|_| ConversionError::FeltConversion)?;
        let oracle_felt =
            cairo_short_string_to_felt(oracle_name).map_err(|_| ConversionError::FeltConversion)?;

        let market_hex = format!("{market_felt:x}");
        let oracle_hex = format!("{oracle_felt:x}");

        Ok(format!("{market_hex:0<32}{oracle_hex:0<8}00"))
    }
    /// Builds the first number for the hash computation based on oracle name and pair id.
    pub fn build_external_asset_id(
        oracle_name: &str,
        pair_id: &str,
    ) -> Result<Felt, ConversionError> {
        let external_asset_id = Self::get_oracle_asset_id(oracle_name, pair_id)?;
        Felt::from_hex(&external_asset_id).map_err(|_| ConversionError::FeltConversion)
    }

    /// Builds the second number for the hash computation based on timestamp and price.
    pub fn build_second_number(
        timestamp: u128,
        price: &BigDecimal,
    ) -> Result<Felt, ConversionError> {
        let price = price.to_u128().ok_or(ConversionError::U128Conversion)?;
        let price_as_hex = format!("{price:x}");
        let timestamp_as_hex = format!("{timestamp:x}");
        let v = format!("{price_as_hex}{timestamp_as_hex}");
        Felt::from_hex(&v).map_err(|_| ConversionError::FeltConversion)
    }
}

impl Signable for EntryComponent {
    /// Computes a signature-ready message based on oracle, asset, timestamp
    /// and price.
    /// The signature is the pedersen hash of two `FieldElements`:
    ///
    /// first number (`oracle_asset_id`):
    ///  ---------------------------------------------------------------------------------
    ///  | `asset_name` (rest of the number)  - 211 bits       |   `oracle_name` (40 bits)   |
    ///  ---------------------------------------------------------------------------------
    ///
    /// second number:
    ///  ---------------------------------------------------------------------------------
    ///  | 0 (92 bits)         | price (120 bits)              |   timestamp (32 bits)   |
    ///  ---------------------------------------------------------------------------------
    ///
    /// See:
    /// <https://docs.starkware.co/starkex/perpetual/becoming-an-oracle-provider-for-starkex.html#signing_prices>
    fn try_get_hash(&self) -> Result<Felt, ConversionError> {
        let first_number =
            Self::build_external_asset_id(&PRAGMA_ORACLE_NAME_FOR_STARKEX, &self.pair_id)?;
        let timestamp = self
            .timestamp
            .parse::<u128>()
            .map_err(|_| ConversionError::StringTimestampConversion)?;
        let second_number = Self::build_second_number(timestamp, &self.price)?;
        Ok(pedersen_hash(&first_number, &second_number))
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MedianEntryWithComponents {
    pub pair_id: String,
    pub median_price: BigDecimal,
    pub components: Vec<EntryComponent>,
}

impl AssetOraclePrice {
    pub fn from_median_entry_with_components(
        median_entry: MedianEntryWithComponents,
        pragma_signer: &SigningKey,
    ) -> Result<Self, ConversionError> {
        let signed_prices: Vec<SignedPublisherPrice> = median_entry
            .components
            .into_iter()
            .map(|component| SignedPublisherPrice::from_entry_component(component, pragma_signer))
            .collect::<Result<Vec<_>, EntryError>>() // Collect into Result<Vec<_>, EntryError>
            .map_err(|_| ConversionError::FeltConversion)?;

        let global_asset_id = StarkexPrice::get_global_asset_id(&median_entry.pair_id)?;

        Ok(Self {
            global_asset_id: format!("0x{global_asset_id}"),
            median_price: median_entry.median_price.to_string(),
            signed_prices: signed_prices,
            signature: Default::default(),
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

pub async fn get_price_with_components(
    pool: &deadpool_diesel::postgres::Pool,
    pair_ids: Vec<String>,
) -> Result<HashMap<String, MedianEntry>, InfraError> {
    let conn = pool.get().await.map_err(InfraError::DbPoolError)?;

    // Handle empty input case
    if pair_ids.is_empty() {
        return Ok(HashMap::new());
    }

    // Determine the table to query based on whether pairs are MARK type
    // We assume the caller has properly formatted the pairs (either all spot or all mark)
    let table_name = if pair_ids.first().map_or(false, |p| p.ends_with(":MARK")) {
        "median_100_ms_perp"
    } else {
        "median_100_ms_spot"
    };

    // Use a window function to get only the latest entry for each pair_id
    let sql_request = format!(
        r"
        WITH ranked_entries AS (
            SELECT
                pair_id,
                bucket AS time,
                median_price,
                num_sources,
                components,
                ROW_NUMBER() OVER (PARTITION BY pair_id ORDER BY bucket DESC) as rn
            FROM
                {}
            WHERE
                pair_id = ANY($1)
        )
        SELECT
            pair_id,
            time,
            median_price,
            num_sources,
            components
        FROM
            ranked_entries
        WHERE
            rn = 1
        ",
        table_name
    );

    // For MARK pairs, we need to strip the suffix for the database query
    let query_pair_ids = if table_name == "median_100_ms_perp" {
        pair_ids
            .iter()
            .map(|p| p.trim_end_matches(":MARK").to_string())
            .collect()
    } else {
        pair_ids.clone()
    };

    let raw_entries = conn
        .interact(move |conn| {
            diesel::sql_query(&sql_request)
                .bind::<diesel::sql_types::Array<diesel::sql_types::Text>, _>(query_pair_ids)
                .load::<ExtendedMedianEntryRaw>(conn)
        })
        .await
        .map_err(InfraError::DbInteractionError)?
        .map_err(InfraError::DbResultError)?;

    // Build result HashMap - for perp pairs, we need to add back the MARK suffix
    let mut result = HashMap::with_capacity(raw_entries.len());
    for raw_entry in raw_entries {
        let components = (!raw_entry.components.is_empty()).then(|| {
            raw_entry
                .components
                .iter()
                .map(ComponentConverter::to_component)
                .collect()
        });

        let entry = MedianEntry {
            time: raw_entry.time,
            median_price: raw_entry.median_price,
            num_sources: raw_entry.num_sources,
            components,
        };

        // For perp table results, add the MARK suffix back to the pair_id
        let key = if table_name == "median_100_ms_perp" {
            format!("{}:MARK", raw_entry.pair_id)
        } else {
            raw_entry.pair_id
        };

        result.insert(key, entry);
    }

    Ok(result)
}
