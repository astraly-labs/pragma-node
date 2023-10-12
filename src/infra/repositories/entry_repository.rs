use bigdecimal::{BigDecimal, ToPrimitive};
use chrono::NaiveDateTime;
use diesel::{
    ExpressionMethods, Insertable, PgTextExpressionMethods, QueryDsl, Queryable, RunQueryDsl,
    Selectable, SelectableHelper,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::models::entry::EntryModel;
use crate::infra::db::schema::entries;
use crate::infra::errors::{adapt_infra_error, InfraError};

#[derive(Serialize, Queryable, Selectable)]
#[diesel(table_name = entries)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct EntryDb {
    pub id: Uuid,
    pub pair_id: String,
    pub publisher: String,
    pub source: String,
    pub timestamp: NaiveDateTime,
    pub price: BigDecimal,
}

#[derive(Deserialize, Insertable)]
#[diesel(table_name = entries)]
pub struct NewEntryDb {
    pub pair_id: String,
    pub publisher: String,
    pub source: String,
    pub timestamp: NaiveDateTime,
    pub price: BigDecimal,
}

#[derive(Deserialize)]
pub struct EntriesFilter {
    pair_id: Option<String>,
    publisher_contains: Option<String>,
}

pub async fn insert(
    pool: &deadpool_diesel::postgres::Pool,
    new_entry: NewEntryDb,
) -> Result<EntryModel, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let res = conn
        .interact(|conn| {
            diesel::insert_into(entries::table)
                .values(new_entry)
                .returning(EntryDb::as_returning())
                .get_result(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    Ok(adapt_entry_db_to_entry(res))
}

pub async fn get(
    pool: &deadpool_diesel::postgres::Pool,
    pair_id: String,
) -> Result<EntryModel, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let res = conn
        .interact(move |conn| {
            entries::table
                .filter(entries::pair_id.eq(pair_id))
                .select(EntryDb::as_select())
                .get_result(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    Ok(adapt_entry_db_to_entry(res))
}

pub async fn _get_all(
    pool: &deadpool_diesel::postgres::Pool,
    filter: EntriesFilter,
) -> Result<Vec<EntryModel>, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let res = conn
        .interact(move |conn| {
            let mut query = entries::table.into_boxed::<diesel::pg::Pg>();

            if let Some(pair_id) = filter.pair_id {
                query = query.filter(entries::pair_id.eq(pair_id));
            }

            if let Some(publisher_contains) = filter.publisher_contains {
                query = query.filter(entries::publisher.ilike(format!("%{}%", publisher_contains)));
            }

            query.select(EntryDb::as_select()).load::<EntryDb>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    let entries: Vec<EntryModel> = res
        .into_iter()
        .map(|entry_db| adapt_entry_db_to_entry(entry_db))
        .collect();

    Ok(entries)
}

fn adapt_entry_db_to_entry(entry_db: EntryDb) -> EntryModel {
    EntryModel {
        id: entry_db.id,
        pair_id: entry_db.pair_id,
        publisher: entry_db.publisher,
        source: entry_db.source,
        timestamp: entry_db.timestamp.timestamp() as u64,
        price: entry_db.price.to_u128().unwrap(), // TODO: remove unwrap
    }
}
