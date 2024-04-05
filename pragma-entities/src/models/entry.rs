use crate::dto::entry as dto;
use crate::models::DieselResult;
use crate::schema::{entries, future_entry, spot_entry};
use bigdecimal::BigDecimal;
use diesel::internal::derives::multiconnection::chrono::NaiveDateTime;
use diesel::upsert::excluded;
use diesel::{
    AsChangeset, ExpressionMethods, Insertable, PgConnection, PgTextExpressionMethods, QueryDsl,
    Queryable, QueryableByName, RunQueryDsl, Selectable, SelectableHelper,
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
    pub price: BigDecimal,
}

#[derive(Serialize, Deserialize, Insertable, AsChangeset)]
#[diesel(table_name = entries)]
pub struct NewEntry {
    pub pair_id: String,
    pub publisher: String,
    pub source: String,
    pub timestamp: NaiveDateTime,
    pub price: BigDecimal,
}

#[derive(Debug, Queryable, Selectable, QueryableByName)]
#[diesel(table_name = spot_entry)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct SpotEntry {
    pub network: String,
    pub pair_id: String,
    pub data_id: String,
    pub block_hash: String,
    pub block_number: i64,
    pub block_timestamp: NaiveDateTime,
    pub transaction_hash: String,
    pub price: BigDecimal,
    pub timestamp: chrono::NaiveDateTime,
    pub publisher: String,
    pub source: String,
    pub volume: BigDecimal,
    pub _cursor: i64,
}

#[derive(Debug, Queryable, Selectable, QueryableByName)]
#[diesel(table_name = future_entry)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FutureEntry {
    pub network: String,
    pub pair_id: String,
    pub data_id: String,
    pub block_hash: String,
    pub block_number: i64,
    pub block_timestamp: NaiveDateTime,
    pub transaction_hash: String,
    pub price: BigDecimal,
    pub timestamp: chrono::NaiveDateTime,
    pub publisher: String,
    pub source: String,
    pub volume: BigDecimal,
    pub expiration_timestamp: Option<chrono::NaiveDateTime>,
    pub _cursor: i64,
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

impl SpotEntry {
    pub fn exists(conn: &mut PgConnection, data_id: String) -> DieselResult<bool> {
        diesel::select(diesel::dsl::exists(
            spot_entry::table.filter(spot_entry::data_id.eq(data_id)),
        ))
        .get_result(conn)
    }

    pub fn get_by_pair_id(conn: &mut PgConnection, pair_id: String) -> DieselResult<Entry> {
        entries::table
            .filter(entries::pair_id.eq(pair_id))
            .select(Entry::as_select())
            .get_result(conn)
    }

    pub fn get_by_data_id(conn: &mut PgConnection, data_id: String) -> DieselResult<SpotEntry> {
        spot_entry::table
            .filter(spot_entry::data_id.eq(data_id))
            .select(SpotEntry::as_select())
            .get_result(conn)
    }
}

impl FutureEntry {
    pub fn exists(conn: &mut PgConnection, data_id: String) -> DieselResult<bool> {
        diesel::select(diesel::dsl::exists(
            future_entry::table.filter(future_entry::data_id.eq(data_id)),
        ))
        .get_result(conn)
    }

    pub fn get_by_pair_id(conn: &mut PgConnection, pair_id: String) -> DieselResult<Entry> {
        entries::table
            .filter(entries::pair_id.eq(pair_id))
            .select(Entry::as_select())
            .get_result(conn)
    }

    pub fn get_by_data_id(conn: &mut PgConnection, data_id: String) -> DieselResult<FutureEntry> {
        future_entry::table
            .filter(future_entry::data_id.eq(data_id))
            .select(FutureEntry::as_select())
            .get_result(conn)
    }
}
