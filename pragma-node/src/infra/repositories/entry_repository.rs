use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use chrono::{DateTime, NaiveDateTime};
use diesel::prelude::QueryableByName;
use diesel::sql_types::{Double, Jsonb, VarChar};
use diesel::{ExpressionMethods, QueryDsl, Queryable, RunQueryDsl};
use serde::{Deserialize, Serialize};
use starknet::core::utils::cairo_short_string_to_felt;
use std::collections::{HashMap, HashSet};

use pragma_common::types::{AggregationMode, Interval};
use pragma_entities::dto;
use pragma_entities::{
    error::{adapt_infra_error, InfraError},
    schema::currencies,
    Currency, Entry, NewEntry,
};

use crate::utils::{convert_via_quote, normalize_to_decimals};

#[derive(Deserialize)]
#[allow(unused)]
pub struct EntriesFilter {
    pair_id: Option<String>,
    publisher_contains: Option<String>,
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

pub async fn routing(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
    interval: Interval,
    timestamp: u64,
    is_routing: bool,
    agg_mode: AggregationMode,
) -> Result<(MedianEntry, u32), InfraError> {
    if pair_id_exist(pool, pair_id.clone()).await? || !is_routing {
        return get_price_decimals(pool, pair_id, interval, timestamp, agg_mode).await;
    }

    let [base, quote]: [&str; 2] = pair_id
        .split('/')
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| InfraError::InternalServerError)?;

    match find_alternative_pair_price(pool, base, quote, interval, timestamp, agg_mode).await {
        Ok(result) => Ok(result),
        Err(_) => Err(InfraError::NotFound),
    }
}

fn calculate_rebased_price(
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
    let min_timestamp = std::cmp::max(
        base_entry.time.and_utc().timestamp(),
        quote_entry.time.and_utc().timestamp(),
    );
    let num_sources = std::cmp::max(base_entry.num_sources, quote_entry.num_sources);
    let new_timestamp = DateTime::from_timestamp(min_timestamp, 0)
        .ok_or(InfraError::InvalidTimeStamp)?
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
    base: &str,
    quote: &str,
    interval: Interval,
    timestamp: u64,
    agg_mode: AggregationMode,
) -> Result<(MedianEntry, u32), InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;

    let alternative_currencies = conn
        .interact(Currency::get_abstract_all)
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    for alt_currency in alternative_currencies {
        let base_alt_pair = format!("{}/{}", base, alt_currency);
        let alt_quote_pair = format!("{}/{}", quote, alt_currency);

        if pair_id_exist(pool, base_alt_pair.clone()).await?
            && pair_id_exist(pool, alt_quote_pair.clone()).await?
        {
            let base_alt_result =
                get_price_decimals(pool, base_alt_pair, interval, timestamp, agg_mode).await?;
            let alt_quote_result =
                get_price_decimals(pool, alt_quote_pair, interval, timestamp, agg_mode).await?;

            return calculate_rebased_price(base_alt_result, alt_quote_result);
        }
    }

    Err(InfraError::NotFound)
}

async fn pair_id_exist(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
) -> Result<bool, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;

    let res = conn
        .interact(move |conn| Entry::exists(conn, pair_id))
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    Ok(res)
}

async fn get_price_decimals(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
    interval: Interval,
    timestamp: u64,
    agg_mode: AggregationMode,
) -> Result<(MedianEntry, u32), InfraError> {
    let entry = match agg_mode {
        AggregationMode::Median => {
            get_median_price(pool, pair_id.clone(), interval, timestamp).await?
        }
        AggregationMode::Twap => get_twap_price(pool, pair_id.clone(), interval, timestamp).await?,
        AggregationMode::Mean => Err(InfraError::InternalServerError)?,
    };

    let decimals = get_decimals(pool, &pair_id).await?;

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
    interval: Interval,
    time: u64,
) -> Result<MedianEntry, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;

    let raw_sql = match interval {
        Interval::OneMinute => {
            r#"
        -- query the materialized realtime view
        SELECT
            bucket AS time,
            price_twap AS median_price,
            num_sources
        FROM
            twap_1_min_agg
        WHERE
            pair_id = $1
            AND
            bucket <= $2
        ORDER BY
            time DESC
        LIMIT 1;
    "#
        }
        Interval::FifteenMinutes => {
            r#"
        -- query the materialized realtime view
        SELECT
            bucket AS time,
            price_twap AS median_price,
            num_sources
        FROM
            twap_15_min_agg
        WHERE
            pair_id = $1
            AND
            bucket <= $2
        ORDER BY
            time DESC
        LIMIT 1;
    "#
        }
        Interval::OneHour => {
            r#"
        -- query the materialized realtime view
        SELECT
            bucket AS time,
            price_twap AS median_price,
            num_sources
        FROM
            twap_1_hour_agg
        WHERE
            pair_id = $1
            AND
            bucket <= $2
        ORDER BY
            time DESC
        LIMIT 1;
    "#
        }
        Interval::TwoHours => {
            r#"
            -- query the materialized realtime view
        SELECT
            bucket AS time,
            price_twap AS median_price,
            num_sources
        FROM
            twap_2_hours_agg
        WHERE
            pair_id = $1
            AND
            bucket <= $2
        ORDER BY
            time DESC
        LIMIT 1;
    "#
        }
    };

    let date_time = DateTime::from_timestamp(time as i64, 0).ok_or(InfraError::InvalidTimeStamp)?;

    let raw_entry = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql)
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
    interval: Interval,
    time: u64,
) -> Result<MedianEntry, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;

    let raw_sql = match interval {
        Interval::OneMinute => {
            r#"
        -- query the materialized realtime view
        SELECT
            bucket AS time,
            median_price,
            num_sources
        FROM
            price_1_min_agg
        WHERE
            pair_id = $1
            AND
            bucket <= $2
        ORDER BY
            time DESC
        LIMIT 1;
    "#
        }
        Interval::FifteenMinutes => {
            r#"
        -- query the materialized realtime view
        SELECT
            bucket AS time,
            median_price,
            num_sources
        FROM
            price_15_min_agg
        WHERE
            pair_id = $1
            AND
            bucket <= $2
        ORDER BY
            time DESC
        LIMIT 1;
    "#
        }
        Interval::OneHour => {
            r#"
        -- query the materialized realtime view
        SELECT
            bucket AS time,
            median_price,
            num_sources
        FROM
            price_1_h_agg
        WHERE
            pair_id = $1
            AND
            bucket <= $2
        ORDER BY
            time DESC
        LIMIT 1;
    "#
        }
        Interval::TwoHours => {
            r#"
            -- query the materialized realtime view
        SELECT
            bucket AS time,
            median_price,
            num_sources
        FROM
            price_2_h_agg
        WHERE
            pair_id = $1
            AND
            bucket <= $2
        ORDER BY
            time DESC
        LIMIT 1;
    "#
        }
    };

    let date_time = DateTime::from_timestamp(time as i64, 0).ok_or(InfraError::InvalidTimeStamp)?;

    let raw_entry = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql)
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

pub async fn get_entries_between(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
    start_timestamp: u64,
    end_timestamp: u64,
) -> Result<Vec<MedianEntry>, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let start_datetime =
        DateTime::from_timestamp(start_timestamp as i64, 0).ok_or(InfraError::InvalidTimeStamp)?;
    let end_datetime =
        DateTime::from_timestamp(end_timestamp as i64, 0).ok_or(InfraError::InvalidTimeStamp)?;

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

pub async fn get_decimals(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: &str,
) -> Result<u32, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;

    let quote_currency = pair_id.split('/').last().unwrap().to_uppercase();
    let base_currency = pair_id.split('/').next().unwrap().to_uppercase();

    // Fetch currency in DB
    let quote_decimals: BigDecimal = conn
        .interact(move |conn| {
            currencies::table
                .filter(currencies::name.eq(quote_currency))
                .select(currencies::decimals)
                .first::<BigDecimal>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;
    let base_decimals: BigDecimal = conn
        .interact(move |conn| {
            currencies::table
                .filter(currencies::name.eq(base_currency))
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

#[derive(Debug, Clone, Serialize, Deserialize, Queryable)]
pub struct OHLCEntry {
    pub time: NaiveDateTime,
    pub open: BigDecimal,
    pub low: BigDecimal,
    pub high: BigDecimal,
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
    time: u64,
) -> Result<Vec<OHLCEntry>, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;

    let raw_sql = match interval {
        Interval::OneMinute => {
            r#"
        -- query the materialized realtime view
        SELECT
            bucket AS time,
            open,
            high,
            low,
            close
        FROM
            one_minute_candle
        WHERE
            pair_id = $1
            AND
            bucket <= $2
        ORDER BY
            time DESC
        LIMIT 10000;
    "#
        }
        Interval::FifteenMinutes => {
            r#"
        -- query the materialized realtime view
        SELECT
            bucket AS time,
            open,
            high,
            low,
            close
        FROM
            fifteen_minute_candle
        WHERE
            pair_id = $1
            AND
            bucket <= $2
        ORDER BY
            time DESC
        LIMIT 10000;
    "#
        }
        Interval::OneHour => {
            r#"
        -- query the materialized realtime view
        SELECT
            bucket AS time,
            open,
            high,
            low,
            close
        FROM
            one_hour_candle
        WHERE
            pair_id = $1
            AND
            bucket <= $2
        ORDER BY
            time DESC
        LIMIT 10000;
    "#
        }
        Interval::TwoHours => {
            r#"
            -- query the materialized realtime view
        SELECT
            bucket AS time,
            open,
            high,
            low,
            close
        FROM
            two_hour_candle
        WHERE
            pair_id = $1
            AND
            bucket <= $2
        ORDER BY
            time DESC
        LIMIT 10000;
    "#
        }
    };

    let date_time = DateTime::from_timestamp(time as i64, 0).ok_or(InfraError::InvalidTimeStamp)?;

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

#[derive(Debug)]
pub enum ConversionError {
    FailedSerialization,
    InvalidDateTime,
    BigDecimalConversion,
}

impl TryFrom<RawMedianEntryWithComponents> for MedianEntryWithComponents {
    type Error = ConversionError;

    fn try_from(raw: RawMedianEntryWithComponents) -> Result<Self, Self::Error> {
        let components: Vec<EntryComponent> =
            serde_json::from_value(raw.components).map_err(|_| Self::Error::FailedSerialization)?;

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

        let pair_id = cairo_short_string_to_felt(&raw.pair_id)
            .map_err(|_| Self::Error::FailedSerialization)?;
        let median_price =
            BigDecimal::from_f64(raw.median_price).ok_or(Self::Error::BigDecimalConversion)?;

        Ok(MedianEntryWithComponents {
            pair_id: format!("0x{:x}", pair_id),
            median_price,
            components,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EntryComponent {
    pub pair_id: String,
    pub price: BigDecimal,
    pub timestamp: String,
    pub publisher: String,
    pub publisher_address: String,
    pub publisher_signature: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MedianEntryWithComponents {
    pub pair_id: String,
    pub median_price: BigDecimal,
    pub components: Vec<EntryComponent>,
}

/// Convert a list of raw entries into a list of valid median entries
/// if the raw entries are valid.
/// The entries are considered valid if:
/// - not empty,
/// - contains at a median price for each pair_id,
/// - each median price has at least 3 unique publishers.
fn get_median_entries_response(
    raw_entries: Vec<RawMedianEntryWithComponents>,
    pairs_ids: &[String],
) -> Option<Vec<MedianEntryWithComponents>> {
    if raw_entries.is_empty() {
        return None;
    }
    let pairs_set: HashSet<_> = pairs_ids.iter().collect();
    let mut found_pairs = HashSet::new();

    let mut median_entries = Vec::with_capacity(raw_entries.len());
    for raw_entry in raw_entries {
        found_pairs.insert(raw_entry.pair_id.clone());

        let median_entry = MedianEntryWithComponents::try_from(raw_entry);
        let median_entry = match median_entry {
            Ok(median_entry) => median_entry,
            Err(_) => panic!("Converting raw entry to median entry failed - should not happen!"),
        };

        let num_unique_publishers = median_entry
            .components
            .iter()
            .map(|c| &c.publisher)
            .collect::<HashSet<_>>()
            .len();
        // TODO(akhercha): Update this to 3 before final push!!!
        if num_unique_publishers < 1 {
            return None;
        }

        median_entries.push(median_entry);
    }
    if found_pairs.len() == pairs_set.len() {
        Some(median_entries)
    } else {
        None
    }
}

/// Builds a SQL query that will fetch the recent prices between now and
/// the given interval for each unique tuple (pair_id, publisher, source)
/// and then calculate the median price for each pair_id.
/// We also return in a JSON string the components that were used to calculate
/// the median price.
fn build_sql_query_for_median_with_components(pair_ids: &[String], interval_in_ms: u64) -> String {
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
                    entries e
                JOIN
                    publishers p ON e.publisher = p.name
                WHERE 
                    e.pair_id IN ({pairs_list})
                    AND e.timestamp >= NOW() - INTERVAL '{interval_in_ms} milliseconds'
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
        pairs_list = pair_ids
            .iter()
            .map(|pair_id| format!("'{}'", pair_id))
            .collect::<Vec<String>>()
            .join(", "),
        interval_in_ms = interval_in_ms
    )
}

// TODO(akhercha): sort this out - do we want a limit ?
// TODO(akhercha): What happens then if we still have nothing? Currently we raise error 404 & break the channel.
pub const LIMIT_INTERVAL_IN_MS: u64 = 5000;
pub const INITAL_INTERVAL_IN_MS: u64 = 500;
pub const INTERVAL_INCREMENT_IN_MS: u64 = 500;

/// Compute the median price for each pair_id in the given list of pair_ids
/// over an interval of time.
/// The interval is increased until we have at least 3 unique publishers
/// and at least one entry for each pair_id.
pub async fn get_current_median_entries_with_components(
    pool: &deadpool_diesel::postgres::Pool,
    pair_ids: &[String],
) -> Result<Vec<MedianEntryWithComponents>, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let mut interval_in_ms = INITAL_INTERVAL_IN_MS;
    let median_entries = loop {
        let raw_sql = build_sql_query_for_median_with_components(pair_ids, interval_in_ms);

        let raw_median_entries = conn
            .interact(move |conn| {
                diesel::sql_query(raw_sql).load::<RawMedianEntryWithComponents>(conn)
            })
            .await
            .map_err(adapt_infra_error)?
            .map_err(adapt_infra_error)?;

        match get_median_entries_response(raw_median_entries, pair_ids) {
            Some(median_entries) => break median_entries,
            None => interval_in_ms += INTERVAL_INCREMENT_IN_MS,
        }

        if interval_in_ms >= LIMIT_INTERVAL_IN_MS {
            tracing::info!("Still nothing until {}ms", interval_in_ms);
            return Err(InfraError::NotFound);
        }
    };

    Ok(median_entries)
}
