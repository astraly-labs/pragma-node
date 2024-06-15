use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, ToSchema)]
pub struct Publisher {
    pub id: Uuid,
    pub name: String,
    pub master_key: String,
    pub active_key: String,
    pub account_address: String,
    pub active: bool,
}

#[derive(Deserialize)]
#[allow(unused)]
pub struct PublishersFilter {
    pub is_active: Option<bool>,
    pub name_contains: Option<String>,
}

impl From<crate::Publishers> for Publisher {
    fn from(publisher: crate::Publishers) -> Self {
        Self {
            id: publisher.id,
            name: publisher.name,
            master_key: publisher.master_key,
            active_key: publisher.active_key,
            account_address: publisher.account_address,
            active: publisher.active,
        }
    }
}
