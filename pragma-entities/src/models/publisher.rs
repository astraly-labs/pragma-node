use diesel::PgConnection;
use diesel::{
    ExpressionMethods, Insertable, PgTextExpressionMethods, QueryDsl, Queryable, RunQueryDsl,
    Selectable, SelectableHelper,
};
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::dto::publisher as dto;
use crate::models::DieselResult;
use crate::schema::publishers;

#[derive(Serialize, Queryable, Selectable)]
#[diesel(table_name = publishers)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Publishers {
    pub id: Uuid,
    pub name: String,
    pub master_key: String,
    pub active_key: String,
    pub active: bool,
    pub account_address: String,
}

#[derive(Deserialize, Insertable)]
#[diesel(table_name = publishers)]
pub struct NewPublisher {
    pub name: String,
    pub master_key: String,
    pub active_key: String,
    pub account_address: String,
}

impl Publishers {
    pub fn get_by_name(conn: &mut PgConnection, name: String) -> DieselResult<Publishers> {
        publishers::table
            .filter(publishers::name.eq(name))
            .select(Publishers::as_select())
            .get_result(conn)
    }

    pub fn with_filters(
        conn: &mut PgConnection,
        filters: dto::PublishersFilter,
    ) -> DieselResult<Vec<Publishers>> {
        let mut query = publishers::table.into_boxed::<diesel::pg::Pg>();
        if let Some(is_active) = filters.is_active {
            query = query.filter(publishers::active.eq(is_active));
        }
        if let Some(name_contains) = filters.name_contains {
            query = query.filter(publishers::name.ilike(format!("%{}%", name_contains)));
        }
        query
            .select(Publishers::as_select())
            .load::<Publishers>(conn)
    }

    pub fn get_account_address_by_name(
        conn: &mut PgConnection,
        name: String,
    ) -> DieselResult<String> {
        publishers::table
            .filter(publishers::name.eq(name))
            .select(publishers::account_address)
            .get_result(conn)
    }
}
