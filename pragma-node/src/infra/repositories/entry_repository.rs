use bigdecimal::{BigDecimal, ToPrimitive};
use chrono::{DateTime, NaiveDateTime};
use serde::{Deserialize, Serialize};

use diesel::prelude::QueryableByName;
use diesel::{ExpressionMethods, QueryDsl, Queryable, RunQueryDsl};

use pragma_entities::dto;
use pragma_entities::{
    error::{adapt_infra_error, InfraError},
    schema::currencies,
    Currency, Entry, NewEntry,
};

use crate::handlers::entries::{AggregationMode, Interval};
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
    let start_datetime = DateTime::from_timestamp(start_timestamp as i64, 0)
        .ok_or(InfraError::InvalidTimeStamp)?
        .naive_utc();
    let end_datetime = DateTime::from_timestamp(end_timestamp as i64, 0)
        .ok_or(InfraError::InvalidTimeStamp)?
        .naive_utc();

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
                .bind::<diesel::sql_types::Timestamp, _>(start_datetime)
                .bind::<diesel::sql_types::Timestamp, _>(end_datetime)
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
