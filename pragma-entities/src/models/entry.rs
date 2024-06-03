use crate::dto::entry as dto;
use crate::models::DieselResult;
use crate::schema::entries;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::upsert::excluded;
use diesel::{
    AsChangeset, ExpressionMethods, Insertable, PgConnection, PgTextExpressionMethods, QueryDsl,
    Queryable, RunQueryDsl, Selectable, SelectableHelper,
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
    pub timestamp: DateTime<Utc>,
    pub price: BigDecimal,
}

#[derive(Serialize, Deserialize, Insertable, AsChangeset)]
#[diesel(table_name = entries)]
pub struct NewEntry {
    pub pair_id: String,
    pub publisher: String,
    pub source: String,
    pub timestamp: DateTime<Utc>,
    pub price: BigDecimal,
}

impl Entry {
    pub fn create_one(conn: &mut PgConnection, data: NewEntry) -> DieselResult<Entry> {
        diesel::insert_into(entries::table)
            .values(data)
            .returning(Entry::as_returning())
            .get_result(conn)
    }

    pub fn create_many(conn: &mut PgConnection, data: Vec<NewEntry>) -> DieselResult<Vec<Entry>> {
        diesel::insert_into(entries::table)
            .values(data)
            .returning(Entry::as_returning())
            .on_conflict((entries::pair_id, entries::source, entries::timestamp))
            .do_update()
            .set((
                entries::pair_id.eq(excluded(entries::pair_id)),
                entries::publisher.eq(excluded(entries::publisher)),
                entries::source.eq(excluded(entries::source)),
                entries::timestamp.eq(excluded(entries::timestamp)),
                entries::price.eq(excluded(entries::price)),
            ))
            .get_results(conn)
    }

    pub fn exists(conn: &mut PgConnection, pair_id: String) -> DieselResult<bool> {
        diesel::select(diesel::dsl::exists(
            entries::table.filter(entries::pair_id.eq(pair_id)),
        ))
        .get_result(conn)
    }

    pub fn get_by_pair_id(conn: &mut PgConnection, pair_id: String) -> DieselResult<Entry> {
        entries::table
            .filter(entries::pair_id.eq(pair_id))
            .select(Entry::as_select())
            .get_result(conn)
    }

    pub fn with_filters(
        conn: &mut PgConnection,
        filters: dto::EntriesFilter,
    ) -> DieselResult<Vec<Entry>> {
        let mut query = entries::table.into_boxed::<diesel::pg::Pg>();

        if let Some(pair_id) = filters.pair_id {
            query = query.filter(entries::pair_id.eq(pair_id));
        }

        if let Some(publisher_contains) = filters.publisher_contains {
            query = query.filter(entries::publisher.ilike(format!("%{}%", publisher_contains)));
        }

        query.select(Entry::as_select()).load::<Entry>(conn)
    }
}
