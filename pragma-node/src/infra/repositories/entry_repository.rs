use bigdecimal::{BigDecimal, ToPrimitive};
use chrono::NaiveDateTime;
use diesel::prelude::QueryableByName;
use diesel::{ExpressionMethods, QueryDsl, Queryable, RunQueryDsl};
use serde::{Deserialize, Serialize};

use pragma_entities::dto;
use pragma_entities::{schema::currencies, Entry, NewEntry, error::{InfraError, adapt_infra_error}};

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
    pub source: String,
    pub time: NaiveDateTime,
    pub median_price: BigDecimal,
}

#[derive(Serialize, QueryableByName)]
pub struct MedianEntryRaw {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub source: String,
    #[diesel(sql_type = diesel::sql_types::Timestamp)]
    pub time: NaiveDateTime,
    #[diesel(sql_type = diesel::sql_types::Numeric)]
    pub median_price: BigDecimal,
}

pub async fn get_median_entries(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
) -> Result<Vec<MedianEntry>, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;

    let raw_sql = r#"
        -- select the latest entry for every publisher,source combination
        SELECT
            source,
            MAX(timestamp) AS time,
            (PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY price))::numeric AS median_price
        FROM
            entries
        WHERE
            pair_id = $1
        GROUP BY
            source
        ORDER BY
            source;
    "#;

    let raw_entries: Vec<MedianEntryRaw> = conn
        .interact(move |conn| {
            diesel::sql_query(raw_sql)
                .bind::<diesel::sql_types::Text, _>(pair_id)
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
            source: raw_entry.source,
        })
        .collect();

    Ok(entries)
}

pub async fn get_entries_between(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
    start_timestamp: u64,
    end_timestamp: u64,
) -> Result<Vec<MedianEntry>, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let start_datetime = NaiveDateTime::from_timestamp_opt(start_timestamp as i64, 0)
        .ok_or(InfraError::InvalidTimeStamp)?;
    let end_datetime = NaiveDateTime::from_timestamp_opt(end_timestamp as i64, 0)
        .ok_or(InfraError::InvalidTimeStamp)?;

    let raw_sql = r#"
        SELECT
            source,
            "timestamp" AS "time",
            PERCENTILE_DISC(0.5) WITHIN GROUP(ORDER BY price) AS "median_price"
        FROM entries
        WHERE pair_id = $1
        AND "timestamp" BETWEEN $2 AND $3
        GROUP BY (timestamp, source)
        ORDER BY timestamp ASC;
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
            source: raw_entry.source,
        })
        .collect();

    Ok(entries)
}

pub async fn get_decimals(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: &str,
) -> Result<u32, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;

    let quote_currency = pair_id.split('/').next().unwrap().to_uppercase();

    // Fetch currency in DB
    let decimals: BigDecimal = conn
        .interact(move |conn| {
            currencies::table
                .filter(currencies::name.eq(quote_currency))
                .select(currencies::decimals)
                .first::<BigDecimal>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    Ok(decimals.to_u32().unwrap())
}
