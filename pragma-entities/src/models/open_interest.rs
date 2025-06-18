use chrono::NaiveDateTime;
use diesel::prelude::*;
use pragma_common::Pair;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::open_interest;

#[derive(Debug, Clone, Queryable, Serialize, Deserialize)]
#[diesel(table_name = open_interest)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OpenInterest {
    pub id: Uuid,
    pub source: String,
    pub pair: String,
    #[serde(rename = "open_interest")]
    #[diesel(column_name = "open_interest")]
    pub open_interest_value: f64,
    pub timestamp: NaiveDateTime,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, Insertable, AsChangeset)]
#[diesel(table_name = open_interest)]
pub struct NewOpenInterest {
    pub source: String,
    pub pair: String,
    #[serde(rename = "open_interest")]
    #[diesel(column_name = "open_interest_value")]
    pub open_interest_value: f64,
    pub timestamp: NaiveDateTime,
}

impl OpenInterest {
    pub fn create_many(
        conn: &mut PgConnection,
        new_entries: Vec<NewOpenInterest>,
    ) -> Result<Vec<Self>, diesel::result::Error> {
        diesel::insert_into(open_interest::table)
            .values(&new_entries)
            .get_results(conn)
    }

    pub fn get_latest(
        conn: &mut PgConnection,
        pair: &Pair,
        source: &str,
    ) -> Result<Option<Self>, diesel::result::Error> {
        open_interest::table
            .filter(open_interest::pair.eq(&pair.to_pair_id()))
            .filter(open_interest::source.eq(&source))
            .order(open_interest::timestamp.desc())
            .first(conn)
            .optional()
    }

    pub fn get_at(
        conn: &mut PgConnection,
        pair: &Pair,
        source: &str,
        timestamp: NaiveDateTime,
    ) -> Result<Option<Self>, diesel::result::Error> {
        open_interest::table
            .filter(open_interest::pair.eq(&pair.to_pair_id()))
            .filter(open_interest::source.eq(&source))
            .filter(open_interest::timestamp.le(timestamp))
            .order(open_interest::timestamp.desc())
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
        open_interest::table
            .filter(open_interest::pair.eq(&pair.to_pair_id()))
            .filter(open_interest::source.eq(&source))
            .filter(open_interest::timestamp.between(start, end))
            .order(open_interest::timestamp.asc())
            .load(conn)
    }

    // Transactional versions of the above methods

    pub fn create_many_transactional(
        conn: &mut PgConnection,
        new_entries: Vec<NewOpenInterest>,
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
