use std::collections::{HashMap, HashSet};

use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::prelude::QueryableByName;
use diesel::sql_types::{Double, Jsonb, VarChar};
use diesel::{ExpressionMethods, QueryDsl, Queryable, RunQueryDsl};
use pragma_common::errors::ConversionError;
use pragma_common::types::pair::Pair;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::constants::others::ROUTING_FRESHNESS_THRESHOLD;
use crate::constants::starkex_ws::{
    INITAL_INTERVAL_IN_MS, INTERVAL_INCREMENT_IN_MS, MAX_INTERVAL_WITHOUT_ENTRIES,
    MINIMUM_NUMBER_OF_PUBLISHERS,
};
use crate::handlers::get_entry::RoutingParams;
use crate::handlers::subscribe_to_entry::{AssetOraclePrice, SignedPublisherPrice};
use crate::utils::{convert_via_quote, normalize_to_decimals};
use pragma_common::signing::starkex::StarkexPrice;
use pragma_common::types::{AggregationMode, DataType, Interval};
use pragma_entities::dto;
use pragma_entities::{
    error::{adapt_infra_error, InfraError},
    schema::currencies,
    Currency, Entry, NewEntry,
};

// SQL statement used to filter the expiration timestamp for future entries
fn get_expiration_timestamp_filter(
    data_type: DataType,
    expiry: String,
) -> Result<String, InfraError> {
    match data_type {
        DataType::SpotEntry => Ok(String::default()),
        DataType::FutureEntry if expiry.is_empty() => {
            Ok(String::from("AND\n\t\texpiration_timestamp is null"))
        }
        DataType::FutureEntry if !expiry.is_empty() => {
            Ok(format!("AND\n\texpiration_timestamp = '{}'", expiry))
        }
        _ => Err(InfraError::InternalServerError),
    }
}

// Retrieve the timescale table based on the network and data type.
fn get_table_suffix(data_type: DataType) -> Result<&'static str, InfraError> {
    match data_type {
        DataType::SpotEntry => Ok(""),
        DataType::FutureEntry => Ok("_future"),
        _ => Err(InfraError::InternalServerError),
    }
}

// Retrieve the timeframe specifier based on the interval and aggregation mode.
pub fn get_interval_specifier(
    interval: Interval,
    is_twap: bool,
) -> Result<&'static str, InfraError> {
    match interval {
        Interval::OneMinute => Ok("1_min"),
        Interval::FifteenMinutes => Ok("15_min"),
        Interval::OneHour if is_twap => Ok("1_hour"),
        Interval::OneHour if !is_twap => Ok("1_h"),
        Interval::TwoHours if is_twap => Ok("2_hours"),
        Interval::TwoHours if !is_twap => Ok("2_h"),
        Interval::OneDay => Ok("1_day"),
        Interval::OneWeek => Ok("1_week"),
        _ => Err(InfraError::InternalServerError),
    }
}

pub async fn _insert(
    pool: &deadpool_diesel::postgres::Pool,
    new_entry: NewEntry,
) -> Result<dto::Entry, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let res = conn
        .interact(|conn| Entry::create_one(conn, new_entry))
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)
        .map(dto::Entry::from)?;
    Ok(res)
}

pub async fn _get(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
) -> Result<dto::Entry, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let res = conn
        .interact(move |conn| Entry::get_by_pair_id(conn, pair_id))
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    Ok(dto::Entry::from(res))
}

pub async fn _get_all(
    pool: &deadpool_diesel::postgres::Pool,
    filter: dto::EntriesFilter,
) -> Result<Vec<dto::Entry>, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let res = conn
        .interact(move |conn| Entry::with_filters(conn, filter))
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?
        .into_iter()
        .map(dto::Entry::from)
        .collect();
    Ok(res)
}

#[derive(Debug, Serialize, Queryable)]
pub struct MedianEntry {
    pub time: NaiveDateTime,
    pub median_price: BigDecimal,
    pub num_sources: i64,
}

#[derive(Serialize, QueryableByName, Clone, Debug)]
pub struct MedianEntryRaw {
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub time: NaiveDateTime,
    #[diesel(sql_type = diesel::sql_types::Numeric)]
    pub median_price: BigDecimal,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub num_sources: i64,
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
    routing_params: RoutingParams,
) -> Result<(MedianEntry, u32), InfraError> {
    // If we have entries for the pair_id and the latest entry is fresh enough,
    // Or if we are not routing, we can return the price directly.
    if !is_routing
        || (pair_id_exist(pool, pair).await?
            && get_last_updated_timestamp(pool, pair.to_pair_id())
                .await?
                .unwrap_or(NaiveDateTime::default())
                .and_utc()
                .timestamp()
                >= Utc::now().naive_utc().and_utc().timestamp() - ROUTING_FRESHNESS_THRESHOLD)
    {
        return get_price_and_decimals(pool, pair, routing_params).await;
    }

    match find_alternative_pair_price(pool, pair, routing_params).await {
        Ok(result) => Ok(result),
        Err(_) => Err(InfraError::NotFound),
    }
}

pub fn calculate_rebased_price(
    base_result: (MedianEntry, u32),
    quote_result: (MedianEntry, u32),
) -> Result<(MedianEntry, u32), InfraError> {
    let (base_entry, base_decimals) = base_result;
    let (quote_entry, quote_decimals) = quote_result;

    if quote_entry.median_price == BigDecimal::from(0) {
        return Err(InfraError::InternalServerError);
    }

    let (rebase_price, decimals) = if base_decimals < quote_decimals {
        let normalized_base_price =
            normalize_to_decimals(base_entry.median_price, base_decimals, quote_decimals);
        (
            convert_via_quote(
                normalized_base_price,
                quote_entry.median_price,
                quote_decimals,
            )?,
            quote_decimals,
        )
    } else {
        let normalized_quote_price =
            normalize_to_decimals(quote_entry.median_price, quote_decimals, base_decimals);
        (
            convert_via_quote(
                base_entry.median_price,
                normalized_quote_price,
                base_decimals,
            )?,
            base_decimals,
        )
    };
    let max_timestamp = std::cmp::max(
        base_entry.time.and_utc().timestamp(),
        quote_entry.time.and_utc().timestamp(),
    );
    let num_sources = std::cmp::max(base_entry.num_sources, quote_entry.num_sources);
    let new_timestamp = DateTime::from_timestamp(max_timestamp, 0)
        .ok_or(InfraError::InvalidTimestamp(format!(
            "Cannot convert to DateTime: {max_timestamp}"
        )))?
        .naive_utc();

    let median_entry = MedianEntry {
        time: new_timestamp,
        median_price: rebase_price,
        num_sources,
    };

    Ok((median_entry, decimals))
}

async fn find_alternative_pair_price(
    pool: &deadpool_diesel::postgres::Pool,
    pair: &Pair,
    routing_params: RoutingParams,
) -> Result<(MedianEntry, u32), InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;

    let alternative_currencies = conn
        .interact(Currency::get_abstract_all)
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    for alt_currency in alternative_currencies {
        let base_alt_pair = Pair::from((pair.base.clone(), alt_currency.clone()));
        let alt_quote_pair = Pair::from((pair.quote.clone(), alt_currency.clone()));

        if pair_id_exist(pool, &base_alt_pair.clone()).await?
            && pair_id_exist(pool, &alt_quote_pair.clone()).await?
        {
            let base_alt_result =
                get_price_and_decimals(pool, &base_alt_pair, routing_params.clone()).await?;
            let alt_quote_result =
                get_price_and_decimals(pool, &alt_quote_pair, routing_params).await?;

            return calculate_rebased_price(base_alt_result, alt_quote_result);
        }
    }

    Err(InfraError::NotFound)
}

async fn pair_id_exist(
    pool: &deadpool_diesel::postgres::Pool,
    pair: &Pair,
) -> Result<bool, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;

    let pair_str = pair.to_string();
    let res = conn
        .interact(move |conn| Entry::exists(conn, pair_str))
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    Ok(res)
}

async fn get_price_and_decimals(
    pool: &deadpool_diesel::postgres::Pool,
    pair: &Pair,
    routing_params: RoutingParams,
) -> Result<(MedianEntry, u32), InfraError> {
    let entry = match routing_params.aggregation_mode {
        AggregationMode::Median => {
            get_median_price(pool, pair.to_pair_id(), routing_params).await?
        }
        AggregationMode::Twap => get_twap_price(pool, pair.to_pair_id(), routing_params).await?,
        AggregationMode::Mean => Err(InfraError::InternalServerError)?,
    };

    let decimals = get_decimals(pool, pair).await?;

    Ok((entry, decimals))
}

pub async fn get_all_currencies_decimals(
    pool: &deadpool_diesel::postgres::Pool,
) -> Result<HashMap<String, BigDecimal>, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let result_vec = conn
        .interact(Currency::get_decimals_all)
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    let mut currencies_decimals_map = HashMap::new();
    for (name, decimals) in result_vec {
        currencies_decimals_map.insert(name, decimals);
    }

    Ok(currencies_decimals_map)
}

pub async fn get_twap_price(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
    routing_params: RoutingParams,
) -> Result<MedianEntry, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;

    let sql_request: String = format!(
        r#"
        -- query the materialized realtime view
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
            {}
        ORDER BY
            time DESC
        LIMIT 1;
    "#,
        get_interval_specifier(routing_params.interval, true)?,
        get_table_suffix(routing_params.data_type)?,
        get_expiration_timestamp_filter(routing_params.data_type, routing_params.expiry)?,
    );

    let date_time = DateTime::from_timestamp(routing_params.timestamp, 0).ok_or(
        InfraError::InvalidTimestamp(format!(
            "Cannot convert to DateTime: {}",
            routing_params.timestamp
        )),
    )?;

    let raw_entry = conn
        .interact(move |conn| {
            diesel::sql_query(&sql_request)
                .bind::<diesel::sql_types::Text, _>(pair_id)
                .bind::<diesel::sql_types::Timestamptz, _>(date_time)
                .load::<MedianEntryRaw>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    let raw_entry = raw_entry.first().ok_or(InfraError::NotFound)?;

    let entry: MedianEntry = MedianEntry {
        time: raw_entry.time,
        median_price: raw_entry.median_price.clone(),
        num_sources: raw_entry.num_sources,
    };

    Ok(entry)
}

pub async fn get_median_price(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
    routing_params: RoutingParams,
) -> Result<MedianEntry, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;

    let sql_request: String = format!(
        r#"
        -- query the materialized realtime view
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
            {}
        ORDER BY
            time DESC
        LIMIT 1;
    "#,
        get_interval_specifier(routing_params.interval, false)?,
        get_table_suffix(routing_params.data_type)?,
        get_expiration_timestamp_filter(routing_params.data_type, routing_params.expiry)?,
    );

    let date_time = DateTime::from_timestamp(routing_params.timestamp, 0).ok_or(
        InfraError::InvalidTimestamp(format!(
            "Cannot convert to DateTime: {}",
            routing_params.timestamp
        )),
    )?;

    let raw_entry = conn
        .interact(move |conn| {
            diesel::sql_query(&sql_request)
                .bind::<diesel::sql_types::Text, _>(pair_id)
                .bind::<diesel::sql_types::Timestamptz, _>(date_time)
                .load::<MedianEntryRaw>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    let raw_entry = raw_entry.first().ok_or(InfraError::NotFound)?;

    let entry: MedianEntry = MedianEntry {
        time: raw_entry.time,
        median_price: raw_entry.median_price.clone(),
        num_sources: raw_entry.num_sources,
    };

    Ok(entry)
}

pub async fn get_median_entries_1_min_between(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
    start_timestamp: u64,
    end_timestamp: u64,
) -> Result<Vec<MedianEntry>, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let start_datetime = DateTime::from_timestamp(start_timestamp as i64, 0).ok_or(
        InfraError::InvalidTimestamp(format!("Cannot convert to DateTime: {start_timestamp}")),
    )?;
    let end_datetime = DateTime::from_timestamp(end_timestamp as i64, 0).ok_or(
        InfraError::InvalidTimestamp(format!("Cannot convert to DateTime: {start_timestamp}")),
    )?;

    let raw_sql = r#"
        SELECT
            bucket AS time,
            median_price,
            num_sources
        FROM price_1_min_agg
        WHERE 
            pair_id = $1
        AND 
            time BETWEEN $2 AND $3
        ORDER BY 
            time DESC;
    "#;

    let raw_entries = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql)
                .bind::<diesel::sql_types::Text, _>(pair_id)
                .bind::<diesel::sql_types::Timestamptz, _>(start_datetime)
                .bind::<diesel::sql_types::Timestamptz, _>(end_datetime)
                .load::<MedianEntryRaw>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    let entries: Vec<MedianEntry> = raw_entries
        .into_iter()
        .map(|raw_entry| MedianEntry {
            time: raw_entry.time,
            median_price: raw_entry.median_price,
            num_sources: raw_entry.num_sources,
        })
        .collect();

    Ok(entries)
}

pub async fn get_median_prices_between(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
    routing_params: RoutingParams,
    start_timestamp: u64,
    end_timestamp: u64,
) -> Result<Vec<MedianEntry>, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let start_datetime = DateTime::from_timestamp(start_timestamp as i64, 0).ok_or(
        InfraError::InvalidTimestamp(format!("Cannot convert to DateTime: {start_timestamp}")),
    )?;
    let end_datetime = DateTime::from_timestamp(end_timestamp as i64, 0).ok_or(
        InfraError::InvalidTimestamp(format!("Cannot convert to DateTime: {end_timestamp}")),
    )?;

    let sql_request: String = format!(
        r#"
        -- query the materialized realtime view
        SELECT
            bucket AS time,
            median_price,
            num_sources
        FROM
            price_{}_agg{}
        WHERE
            pair_id = $1
            AND
            bucket BETWEEN $2 AND $3
            {}
        ORDER BY
            time DESC;
    "#,
        get_interval_specifier(routing_params.interval, false)?,
        get_table_suffix(routing_params.data_type)?,
        get_expiration_timestamp_filter(routing_params.data_type, routing_params.expiry)?,
    );

    let raw_entries = conn
        .interact(move |conn| {
            diesel::sql_query(&sql_request)
                .bind::<diesel::sql_types::Text, _>(pair_id)
                .bind::<diesel::sql_types::Timestamptz, _>(start_datetime)
                .bind::<diesel::sql_types::Timestamptz, _>(end_datetime)
                .load::<MedianEntryRaw>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    let entries: Vec<MedianEntry> = raw_entries
        .into_iter()
        .map(|raw_entry| MedianEntry {
            time: raw_entry.time,
            median_price: raw_entry.median_price,
            num_sources: raw_entry.num_sources,
        })
        .collect();

    Ok(entries)
}

pub async fn get_twap_prices_between(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
    routing_params: RoutingParams,
    start_timestamp: u64,
    end_timestamp: u64,
) -> Result<Vec<MedianEntry>, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let start_datetime = DateTime::from_timestamp(start_timestamp as i64, 0).ok_or(
        InfraError::InvalidTimestamp(format!("Cannot convert to DateTime: {start_timestamp}")),
    )?;
    let end_datetime = DateTime::from_timestamp(end_timestamp as i64, 0).ok_or(
        InfraError::InvalidTimestamp(format!("Cannot convert to DateTime: {end_timestamp}")),
    )?;

    let sql_request: String = format!(
        r#"
        -- query the materialized realtime view
        SELECT
            bucket AS time,
            price_twap AS median_price,
            num_sources
        FROM
            twap_{}_agg{}
        WHERE
            pair_id = $1
            AND
            bucket BETWEEN $2 AND $3
            {}
        ORDER BY
            time DESC;
    "#,
        get_interval_specifier(routing_params.interval, true)?,
        get_table_suffix(routing_params.data_type)?,
        get_expiration_timestamp_filter(routing_params.data_type, routing_params.expiry)?,
    );

    let raw_entries = conn
        .interact(move |conn| {
            diesel::sql_query(&sql_request)
                .bind::<diesel::sql_types::Text, _>(pair_id)
                .bind::<diesel::sql_types::Timestamptz, _>(start_datetime)
                .bind::<diesel::sql_types::Timestamptz, _>(end_datetime)
                .load::<MedianEntryRaw>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    let entries: Vec<MedianEntry> = raw_entries
        .into_iter()
        .map(|raw_entry| MedianEntry {
            time: raw_entry.time,
            median_price: raw_entry.median_price,
            num_sources: raw_entry.num_sources,
        })
        .collect();

    Ok(entries)
}

pub async fn get_decimals(
    pool: &deadpool_diesel::postgres::Pool,
    pair: &Pair,
) -> Result<u32, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;

    let (quote, base) = pair.as_tuple();

    // Fetch currency in DB
    let quote_decimals: BigDecimal = conn
        .interact(move |conn| {
            currencies::table
                .filter(currencies::name.eq(quote))
                .select(currencies::decimals)
                .first::<BigDecimal>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    let base_decimals: BigDecimal = conn
        .interact(move |conn| {
            currencies::table
                .filter(currencies::name.eq(base))
                .select(currencies::decimals)
                .first::<BigDecimal>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    // Take the minimum of the two
    let decimals = std::cmp::min(
        quote_decimals.to_u32().unwrap(),
        base_decimals.to_u32().unwrap(),
    );

    Ok(decimals)
}

pub async fn get_last_updated_timestamp(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
) -> Result<Option<NaiveDateTime>, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    conn.interact(|conn| Entry::get_last_updated_timestamp(conn, pair_id))
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)
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
        OHLCEntry {
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
    let conn = pool.get().await.map_err(adapt_infra_error)?;

    let raw_sql = format!(
        r#"
        -- query the materialized realtime view
        SELECT
            ohlc_bucket AS time,
            open,
            high,
            low,
            close
        FROM
            new_{}_candle
        WHERE
            pair_id = $1
            AND
            ohlc_bucket <= $2
        ORDER BY
            time DESC
        LIMIT 10000;
    "#,
        get_interval_specifier(interval, false)?
    );

    let date_time = DateTime::from_timestamp(time, 0).ok_or(InfraError::InvalidTimestamp(
        format!("Cannot convert to DateTime: {time}"),
    ))?;

    let raw_entries = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql)
                .bind::<diesel::sql_types::Text, _>(pair_id)
                .bind::<diesel::sql_types::Timestamptz, _>(date_time)
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

        Ok(MedianEntryWithComponents {
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

        // Scale price from 8 decimals to 18 decimals for StarkEx
        let price_with_18_decimals = component.price * BigDecimal::from(10_u64.pow(10));

        Ok(SignedPublisherPrice {
            oracle_asset_id: format!("0x{}", asset_id),
            oracle_price: price_with_18_decimals.to_string(),
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

        // Scale price from 8 decimals to 18 decimals for StarkEx
        let price_with_18_decimals = median_entry.median_price * BigDecimal::from(10_u64.pow(10));

        Ok(AssetOraclePrice {
            global_asset_id: format!("0x{}", global_asset_id),
            median_price: price_with_18_decimals.to_string(),
            signed_prices: signed_prices?,
            signature: Default::default(),
        })
    }
}

/// Convert a list of raw entries into a list of valid median entries.
/// For each pair_id, check if it has a valid median price with enough unique publishers.
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
fn get_table_name_from_type(entry_type: DataType) -> &'static str {
    match entry_type {
        DataType::SpotEntry => "entries",
        DataType::FutureEntry => "future_entries",
        DataType::PerpEntry => "future_entries",
    }
}

/// We exclude PRAGMA publisher for starkex endpoint as data is not reliable for that use case.
/// One solution would be to adapt the price-pusher to push prices as soon as they are available.
/// For now, we prefer to just work with data from 1st party sources.
const EXCLUDED_PUBLISHER: &str = "";

/// Builds a SQL query that will fetch the recent prices between now and
/// the given interval for each unique tuple (pair_id, publisher, source)
/// and then calculate the median price for each pair_id.
/// We also return in a JSON string the components that were used to calculate
/// the median price.
fn build_sql_query_for_median_with_components(
    pair_ids: &[String],
    interval_in_ms: u64,
    entry_type: DataType,
) -> String {
    format!(
        r#"
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
            "#,
        table_name = get_table_name_from_type(entry_type),
        pairs_list = pair_ids
            .iter()
            .map(|pair_id| format!("'{}'", pair_id))
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

/// Compute the median price for each pair_id in the given list of pair_ids
/// over an interval of time.
/// The interval is increased until we have valid entries with enough publishers.
/// Returns any pairs that have valid data, even if some pairs are invalid.
pub async fn get_current_median_entries_with_components(
    pool: &deadpool_diesel::postgres::Pool,
    pair_ids: &[String],
    entry_type: DataType,
) -> Result<Vec<MedianEntryWithComponents>, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
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
            .map_err(adapt_infra_error)?
            .map_err(adapt_infra_error)?;

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
    let conn = pool.get().await.map_err(adapt_infra_error)?;

    let sql_request: String = r#"
        SELECT DISTINCT expiration_timestamp
        FROM future_entries
        WHERE pair_id = $1 AND expiration_timestamp IS NOT NULL
        ORDER BY expiration_timestamp;
        "#
    .to_string();

    let raw_exp = conn
        .interact(move |conn| {
            diesel::sql_query(&sql_request)
                .bind::<diesel::sql_types::Text, _>(pair_id)
                .load::<ExpiriesListRaw>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    let expiries: Vec<NaiveDateTime> = raw_exp
        .into_iter()
        .map(|r| r.expiration_timestamp)
        .collect();

    Ok(expiries)
}
