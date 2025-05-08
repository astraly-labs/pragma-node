use chrono::NaiveDateTime;
use diesel::{prelude::*, sql_types::VarChar};
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
}
