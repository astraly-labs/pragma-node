use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sql_types::{Double, Timestamp, VarChar};
use pragma_common::Pair;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::funding_rates;

#[derive(Debug, Clone, Queryable, Serialize, Deserialize)]
#[diesel(table_name = funding_rates)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FundingRate {
    pub id: Uuid,
    pub source: String,
    pub pair: String,
    pub annualized_rate: f64,
    pub timestamp: NaiveDateTime,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, Insertable, AsChangeset)]
#[diesel(table_name = funding_rates)]
pub struct NewFundingRate {
    pub source: String,
    pub pair: String,
    pub annualized_rate: f64,
    pub timestamp: NaiveDateTime,
}

impl FundingRate {
    pub fn create_many(
        conn: &mut PgConnection,
        new_entries: Vec<NewFundingRate>,
    ) -> Result<Vec<Self>, diesel::result::Error> {
        diesel::insert_into(funding_rates::table)
            .values(&new_entries)
            .get_results(conn)
    }

    pub fn get_latest(
        conn: &mut PgConnection,
        pair: &Pair,
        source: &str,
    ) -> Result<Option<Self>, diesel::result::Error> {
        funding_rates::table
            .filter(funding_rates::pair.eq(&pair.to_pair_id()))
            .filter(funding_rates::source.eq(&source))
            .order(funding_rates::timestamp.desc())
            .first(conn)
            .optional()
    }

    pub fn get_at(
        conn: &mut PgConnection,
        pair: &Pair,
        source: &str,
        timestamp: NaiveDateTime,
    ) -> Result<Option<Self>, diesel::result::Error> {
        funding_rates::table
            .filter(funding_rates::pair.eq(&pair.to_pair_id()))
            .filter(funding_rates::source.eq(&source))
            .filter(funding_rates::timestamp.le(timestamp))
            .order(funding_rates::timestamp.desc())
            .first(conn)
            .optional()
    }

    pub fn get_in_range(
        conn: &mut PgConnection,
        pair: &Pair,
        source: &str,
        start: NaiveDateTime,
        end: NaiveDateTime,
    ) -> Result<Vec<Self>, diesel::result::Error> {
        funding_rates::table
            .filter(funding_rates::pair.eq(&pair.to_pair_id()))
            .filter(funding_rates::source.eq(&source))
            .filter(funding_rates::timestamp.between(start, end))
            .order(funding_rates::timestamp.asc())
            .load(conn)
    }

    pub fn get_in_range_aggregated(
        conn: &mut PgConnection,
        pair: &Pair,
        source: &str,
        start: NaiveDateTime,
        end: NaiveDateTime,
        aggregate_table: &str,
    ) -> Result<Vec<Self>, diesel::result::Error> {
        #[derive(diesel::QueryableByName)]
        struct AggregatedFundingRate {
            #[diesel(sql_type = VarChar)]
            source: String,
            #[diesel(sql_type = VarChar)]
            pair: String,
            #[diesel(sql_type = Double)]
            avg_annualized_rate: f64,
            #[diesel(sql_type = Timestamp)]
            bucket: NaiveDateTime,
        }

        let query = format!(
            r"
            SELECT 
                source,
                pair,
                avg_annualized_rate,
                bucket
            FROM {aggregate_table}
            WHERE pair = $1 
                AND source = $2 
                AND bucket >= $3 
                AND bucket <= $4
            ORDER BY bucket ASC
            "
        );

        let results: Vec<AggregatedFundingRate> = diesel::sql_query(query)
            .bind::<VarChar, _>(&pair.to_pair_id())
            .bind::<VarChar, _>(source)
            .bind::<Timestamp, _>(start)
            .bind::<Timestamp, _>(end)
            .load(conn)?;

        let funding_rates = results
            .into_iter()
            .map(|r| Self {
                id: Uuid::new_v4(), // Generate new UUID for aggregated data
                source: r.source,
                pair: r.pair,
                annualized_rate: r.avg_annualized_rate,
                timestamp: r.bucket,
                created_at: r.bucket, // Use bucket time as created_at for aggregated data
            })
            .collect();

        Ok(funding_rates)
    }

    // Transactional versions of the above methods

    pub fn create_many_transactional(
        conn: &mut PgConnection,
        new_entries: Vec<NewFundingRate>,
    ) -> Result<Vec<Self>, diesel::result::Error> {
        conn.transaction(|conn| Self::create_many(conn, new_entries))
    }

    pub fn get_latest_transactional(
        conn: &mut PgConnection,
        pair: &Pair,
        source: &str,
    ) -> Result<Option<Self>, diesel::result::Error> {
        conn.transaction(|conn| Self::get_latest(conn, pair, source))
    }

    pub fn get_at_transactional(
        conn: &mut PgConnection,
        pair: &Pair,
        source: &str,
        timestamp: NaiveDateTime,
    ) -> Result<Option<Self>, diesel::result::Error> {
        conn.transaction(|conn| Self::get_at(conn, pair, source, timestamp))
    }

    pub fn get_in_range_transactional(
        conn: &mut PgConnection,
        pair: &Pair,
        source: &str,
        start: NaiveDateTime,
        end: NaiveDateTime,
    ) -> Result<Vec<Self>, diesel::result::Error> {
        conn.transaction(|conn| Self::get_in_range(conn, pair, source, start, end))
    }

    pub fn get_in_range_aggregated_transactional(
        conn: &mut PgConnection,
        pair: &Pair,
        source: &str,
        start: NaiveDateTime,
        end: NaiveDateTime,
        aggregate_table: &str,
    ) -> Result<Vec<Self>, diesel::result::Error> {
        conn.transaction(|conn| {
            Self::get_in_range_aggregated(conn, pair, source, start, end, aggregate_table)
        })
    }

    // Batch operations with transactions
    pub fn batch_operations_transactional<F, T>(
        conn: &mut PgConnection,
        operations: F,
    ) -> Result<T, diesel::result::Error>
    where
        F: FnOnce(&mut PgConnection) -> Result<T, diesel::result::Error>,
    {
        conn.transaction(operations)
    }
}
