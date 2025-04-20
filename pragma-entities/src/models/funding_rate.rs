use chrono::NaiveDateTime;
use diesel::prelude::*;
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
}
