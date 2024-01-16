use utoipa::ToSchema;
use uuid::Uuid;
use crate::schema::currencies;
use super::DieselResult;
use diesel::{
    ExpressionMethods, PgConnection, QueryDsl,
    RunQueryDsl,
};

#[derive(Clone, Debug, PartialEq, ToSchema)]
pub struct Currency {
    pub id: Uuid,
    pub name: String,
    pub decimals: u64,
    pub is_abstract: bool,
    pub ethereum_address: String,
}

impl Currency {
    pub fn get_all(conn: &mut PgConnection) -> DieselResult<Vec<String>> {
        currencies::table
            .select(currencies::name)
            .get_results(conn)
    }

    pub fn get_abstract_all(conn: &mut PgConnection) -> DieselResult<Vec<String>> {
        currencies::table
            .select(currencies::name)
            .filter(currencies::abstract_.eq(true))
            .get_results(conn)
    }
}