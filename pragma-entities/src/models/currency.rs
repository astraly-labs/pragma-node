use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, ToSchema)]
pub struct Currency {
    pub id: Uuid,
    pub name: String,
    pub decimals: u64,
    pub is_abstract: bool,
    pub ethereum_address: String,
}
