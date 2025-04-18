use crate::EntryError;
use crate::TimestampError;
use crate::convert_timestamp_to_datetime;
use crate::dto::entry as dto;
use crate::models::DieselResult;
use crate::schema::entries;
use bigdecimal::BigDecimal;
use diesel::internal::derives::multiconnection::chrono::NaiveDateTime;
use diesel::{
    AsChangeset, ExpressionMethods, Insertable, OptionalExtension, PgConnection,
    PgTextExpressionMethods, QueryDsl, Queryable, RunQueryDsl, Selectable, SelectableHelper,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Queryable, Selectable)]
#[diesel(table_name = entries)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Entry {
    pub id: Uuid,
    pub pair_id: String,
    pub publisher: String,
    pub source: String,
    pub timestamp: NaiveDateTime,
    pub publisher_signature: Option<String>,
    pub price: BigDecimal,
}

#[derive(Serialize, Deserialize, Insertable, AsChangeset, Debug)]
#[diesel(table_name = entries)]
pub struct NewEntry {
    pub pair_id: String,
    pub publisher: String,
    pub source: String,
    pub timestamp: NaiveDateTime,
    pub price: BigDecimal,
}

impl Entry {
    pub fn create_one(conn: &mut PgConnection, data: NewEntry) -> DieselResult<Self> {
        diesel::insert_into(entries::table)
            .values(data)
            .returning(Self::as_returning())
            .get_result(conn)
    }

    pub fn create_many(conn: &mut PgConnection, data: Vec<NewEntry>) -> DieselResult<Vec<Self>> {
        diesel::insert_into(entries::table)
            .values(data)
            .returning(Self::as_returning())
            .on_conflict((entries::pair_id, entries::source, entries::timestamp))
            .do_nothing()
            .get_results(conn)
    }

    pub fn exists(conn: &mut PgConnection, pair_id: String) -> DieselResult<bool> {
        diesel::select(diesel::dsl::exists(
            entries::table.filter(entries::pair_id.eq(pair_id)),
        ))
        .get_result(conn)
    }

    pub fn get_by_pair_id(conn: &mut PgConnection, pair_id: String) -> DieselResult<Self> {
        entries::table
            .filter(entries::pair_id.eq(pair_id))
            .select(Self::as_select())
            .get_result(conn)
    }

    pub fn with_filters(
        conn: &mut PgConnection,
        filters: dto::EntriesFilter,
    ) -> DieselResult<Vec<Self>> {
        let mut query = entries::table.into_boxed::<diesel::pg::Pg>();

        if let Some(pair_id) = filters.pair_id {
            query = query.filter(entries::pair_id.eq(pair_id));
        }

        if let Some(publisher_contains) = filters.publisher_contains {
            query = query.filter(entries::publisher.ilike(format!("%{publisher_contains}%")));
        }

        query.select(Self::as_select()).load::<Self>(conn)
    }

    pub fn get_existing_pairs(
        conn: &mut PgConnection,
        searched_pairs: Vec<String>,
    ) -> DieselResult<Vec<String>> {
        entries::table
            .filter(entries::pair_id.eq_any(searched_pairs))
            .select(entries::pair_id)
            .distinct()
            .load::<String>(conn)
    }

    pub fn get_last_updated_timestamp(
        conn: &mut PgConnection,
        pair: String,
        max_timestamp: i64,
    ) -> DieselResult<Option<chrono::NaiveDateTime>> {
        let max_timestamp = convert_timestamp_to_datetime!(max_timestamp).map_err(|_| {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::CheckViolation,
                Box::new(format!("Invalid timestamp value: {max_timestamp}")),
            )
        })?;
        entries::table
            .filter(entries::pair_id.eq(pair))
            .filter(entries::timestamp.le(max_timestamp))
            .select(entries::timestamp)
            .order(entries::timestamp.desc())
            .first(conn)
            .optional()
    }
}
