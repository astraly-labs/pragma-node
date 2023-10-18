use diesel::{
    ExpressionMethods, Insertable, PgTextExpressionMethods, QueryDsl, Queryable, RunQueryDsl,
    Selectable, SelectableHelper,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::models::publisher::PublisherModel;
use crate::infra::db::schema::publishers;
use crate::infra::errors::{adapt_infra_error, InfraError};

#[derive(Serialize, Queryable, Selectable)]
#[diesel(table_name = publishers)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PublisherDb {
    pub id: Uuid,
    pub name: String,
    pub master_key: String,
    pub active_key: String,
    pub active: bool,
    pub account_address: String,
}

#[derive(Deserialize, Insertable)]
#[diesel(table_name = publishers)]
pub struct NewPublisherDb {
    pub name: String,
    pub master_key: String,
    pub active_key: String,
    pub account_address: String,
}

#[derive(Deserialize)]
#[allow(unused)]
pub struct PublishersFilter {
    is_active: Option<bool>,
    name_contains: Option<String>,
}

pub async fn _insert(
    pool: &deadpool_diesel::postgres::Pool,
    new_entry: NewPublisherDb,
) -> Result<PublisherModel, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let res = conn
        .interact(|conn| {
            diesel::insert_into(publishers::table)
                .values(new_entry)
                .returning(PublisherDb::as_returning())
                .get_result(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    Ok(adapt_publisher_db_to_publisher(res))
}

pub async fn get(
    pool: &deadpool_diesel::postgres::Pool,
    name: String,
) -> Result<PublisherModel, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let res = conn
        .interact(move |conn| {
            publishers::table
                .filter(publishers::name.eq(name))
                .select(PublisherDb::as_select())
                .get_result(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    Ok(adapt_publisher_db_to_publisher(res))
}

pub async fn _get_all(
    pool: &deadpool_diesel::postgres::Pool,
    filter: PublishersFilter,
) -> Result<Vec<PublisherModel>, InfraError> {
    let conn = pool.get().await.map_err(adapt_infra_error)?;
    let res = conn
        .interact(move |conn| {
            let mut query = publishers::table.into_boxed::<diesel::pg::Pg>();

            if let Some(is_active) = filter.is_active {
                query = query.filter(publishers::active.eq(is_active));
            }

            if let Some(name_contains) = filter.name_contains {
                query = query.filter(publishers::name.ilike(format!("%{}%", name_contains)));
            }

            query
                .select(PublisherDb::as_select())
                .load::<PublisherDb>(conn)
        })
        .await
        .map_err(adapt_infra_error)?
        .map_err(adapt_infra_error)?;

    let entries: Vec<PublisherModel> = res
        .into_iter()
        .map(adapt_publisher_db_to_publisher)
        .collect();

    Ok(entries)
}

fn adapt_publisher_db_to_publisher(entry_db: PublisherDb) -> PublisherModel {
    PublisherModel {
        id: entry_db.id,
        name: entry_db.name,
        master_key: entry_db.master_key,
        active_key: entry_db.active_key,
        account_address: entry_db.account_address,
        active: entry_db.active,
    }
}
