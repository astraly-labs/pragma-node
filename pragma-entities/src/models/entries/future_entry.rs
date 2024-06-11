use crate::dto::entry as dto;
use crate::models::DieselResult;
use bigdecimal::BigDecimal;
use diesel::internal::derives::multiconnection::chrono::NaiveDateTime;
use diesel::upsert::excluded;
use diesel::{
    AsChangeset, ExpressionMethods, Insertable, PgConnection, PgTextExpressionMethods, QueryDsl,
    Queryable, RunQueryDsl, Selectable, SelectableHelper,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::future_entries;

#[derive(Serialize, Queryable, Selectable)]
#[diesel(table_name = future_entries)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FutureEntry {
    pub id: Uuid,
    pub pair_id: String,
    pub publisher: String,
    pub source: String,
    pub timestamp: NaiveDateTime,
    pub expiration_timestamp: NaiveDateTime,
    pub publisher_signature: String,
    pub price: BigDecimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, Insertable, AsChangeset)]
#[diesel(table_name = future_entries)]
pub struct NewFutureEntry {
    pub pair_id: String,
    pub publisher: String,
    pub source: String,
    pub timestamp: NaiveDateTime,
    pub expiration_timestamp: NaiveDateTime,
    pub publisher_signature: String,
    pub price: BigDecimal,
}

impl FutureEntry {
    pub fn create_one(conn: &mut PgConnection, data: NewFutureEntry) -> DieselResult<FutureEntry> {
        diesel::insert_into(future_entries::table)
            .values(data)
            .returning(FutureEntry::as_returning())
            .get_result(conn)
    }

    pub fn create_many(
        conn: &mut PgConnection,
        data: Vec<NewFutureEntry>,
    ) -> DieselResult<Vec<FutureEntry>> {
        diesel::insert_into(future_entries::table)
            .values(data)
            .returning(FutureEntry::as_returning())
            .on_conflict((
                future_entries::pair_id,
                future_entries::source,
                future_entries::timestamp,
                future_entries::expiration_timestamp,
            ))
            .do_update()
            .set((
                future_entries::pair_id.eq(excluded(future_entries::pair_id)),
                future_entries::publisher.eq(excluded(future_entries::publisher)),
                future_entries::source.eq(excluded(future_entries::source)),
                future_entries::publisher_signature
                    .eq(excluded(future_entries::publisher_signature)),
                future_entries::timestamp.eq(excluded(future_entries::timestamp)),
                future_entries::expiration_timestamp
                    .eq(excluded(future_entries::expiration_timestamp)),
                future_entries::price.eq(excluded(future_entries::price)),
            ))
            .get_results(conn)
    }

    pub fn exists(conn: &mut PgConnection, pair_id: String) -> DieselResult<bool> {
        diesel::select(diesel::dsl::exists(
            future_entries::table.filter(future_entries::pair_id.eq(pair_id)),
        ))
        .get_result(conn)
    }

    pub fn get_by_pair_id(conn: &mut PgConnection, pair_id: String) -> DieselResult<FutureEntry> {
        future_entries::table
            .filter(future_entries::pair_id.eq(pair_id))
            .select(FutureEntry::as_select())
            .get_result(conn)
    }

    pub fn with_filters(
        conn: &mut PgConnection,
        filters: dto::EntriesFilter,
    ) -> DieselResult<Vec<FutureEntry>> {
        let mut query = future_entries::table.into_boxed::<diesel::pg::Pg>();

        if let Some(pair_id) = filters.pair_id {
            query = query.filter(future_entries::pair_id.eq(pair_id));
        }

        if let Some(publisher_contains) = filters.publisher_contains {
            query =
                query.filter(future_entries::publisher.ilike(format!("%{}%", publisher_contains)));
        }

        query
            .select(FutureEntry::as_select())
            .load::<FutureEntry>(conn)
    }

    pub fn get_existing_pairs(
        conn: &mut PgConnection,
        searched_pairs: Vec<String>,
    ) -> DieselResult<Vec<String>> {
        future_entries::table
            .filter(future_entries::pair_id.eq_any(searched_pairs))
            .select(future_entries::pair_id)
            .distinct()
            .load::<String>(conn)
    }
}
