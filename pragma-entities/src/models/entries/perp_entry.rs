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

use crate::schema::perp_entries;

#[derive(Serialize, Queryable, Selectable)]
#[diesel(table_name = perp_entries)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PerpEntry {
    pub id: Uuid,
    pub pair_id: String,
    pub publisher: String,
    pub source: String,
    pub timestamp: NaiveDateTime,
    pub publisher_signature: String,
    pub price: BigDecimal,
}

#[derive(Serialize, Deserialize, Insertable, AsChangeset)]
#[diesel(table_name = perp_entries)]
pub struct NewPerpEntry {
    pub pair_id: String,
    pub publisher: String,
    pub source: String,
    pub timestamp: NaiveDateTime,
    pub publisher_signature: String,
    pub price: BigDecimal,
}

impl PerpEntry {
    pub fn create_one(conn: &mut PgConnection, data: NewPerpEntry) -> DieselResult<PerpEntry> {
        diesel::insert_into(perp_entries::table)
            .values(data)
            .returning(PerpEntry::as_returning())
            .get_result(conn)
    }

    pub fn create_many(
        conn: &mut PgConnection,
        data: Vec<NewPerpEntry>,
    ) -> DieselResult<Vec<PerpEntry>> {
        diesel::insert_into(perp_entries::table)
            .values(data)
            .returning(PerpEntry::as_returning())
            .on_conflict((
                perp_entries::pair_id,
                perp_entries::source,
                perp_entries::timestamp,
            ))
            .do_update()
            .set((
                perp_entries::pair_id.eq(excluded(perp_entries::pair_id)),
                perp_entries::publisher.eq(excluded(perp_entries::publisher)),
                perp_entries::source.eq(excluded(perp_entries::source)),
                perp_entries::publisher_signature.eq(excluded(perp_entries::publisher_signature)),
                perp_entries::timestamp.eq(excluded(perp_entries::timestamp)),
                perp_entries::price.eq(excluded(perp_entries::price)),
            ))
            .get_results(conn)
    }

    pub fn exists(conn: &mut PgConnection, pair_id: String) -> DieselResult<bool> {
        diesel::select(diesel::dsl::exists(
            perp_entries::table.filter(perp_entries::pair_id.eq(pair_id)),
        ))
        .get_result(conn)
    }

    pub fn get_by_pair_id(conn: &mut PgConnection, pair_id: String) -> DieselResult<PerpEntry> {
        perp_entries::table
            .filter(perp_entries::pair_id.eq(pair_id))
            .select(PerpEntry::as_select())
            .get_result(conn)
    }

    pub fn with_filters(
        conn: &mut PgConnection,
        filters: dto::EntriesFilter,
    ) -> DieselResult<Vec<PerpEntry>> {
        let mut query = perp_entries::table.into_boxed::<diesel::pg::Pg>();

        if let Some(pair_id) = filters.pair_id {
            query = query.filter(perp_entries::pair_id.eq(pair_id));
        }

        if let Some(publisher_contains) = filters.publisher_contains {
            query =
                query.filter(perp_entries::publisher.ilike(format!("%{}%", publisher_contains)));
        }

        query.select(PerpEntry::as_select()).load::<PerpEntry>(conn)
    }

    pub fn get_existing_pairs(
        conn: &mut PgConnection,
        searched_pairs: Vec<String>,
    ) -> DieselResult<Vec<String>> {
        perp_entries::table
            .filter(perp_entries::pair_id.eq_any(searched_pairs))
            .select(perp_entries::pair_id)
            .distinct()
            .load::<String>(conn)
    }
}
