use crate::utils::{
    typed_data::{Domain, Field, PrimitiveType, SimpleField},
    TypedData,
};
use indexmap::IndexMap;
use pragma_entities::models::entry_error::SigningError;
use serde::{Deserialize, Serialize};
use starknet::core::types::Felt;
use utoipa::ToSchema;

use crate::utils::felt_from_decimal;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginMessage {
    #[schema(value_type = Vec<String>)]
    #[serde(deserialize_with = "felt_from_decimal")]
    pub signature: Vec<Felt>,
    pub publisher_name: String,
}

pub fn build_login_message(publisher_name: &str) -> Result<TypedData, SigningError> {
    // Define the domain
    let domain = Domain::new("Pragma", "1", "1", Some("1"));

    // Define the types
    let mut types = IndexMap::new();

    // Add "StarknetDomain" type
    types.insert(
        "StarknetDomain".to_string(),
        vec![
            Field::SimpleType(SimpleField {
                name: "name".to_string(),
                r#type: "shortstring".to_string(),
            }),
            Field::SimpleType(SimpleField {
                name: "version".to_string(),
                r#type: "shortstring".to_string(),
            }),
            Field::SimpleType(SimpleField {
                name: "chainId".to_string(),
                r#type: "shortstring".to_string(),
            }),
            Field::SimpleType(SimpleField {
                name: "revision".to_string(),
                r#type: "shortstring".to_string(),
            }),
        ],
    );

    // Add "Request" type
    types.insert(
        "Request".to_string(),
        vec![Field::SimpleType(SimpleField {
            name: "publisher_name".to_string(),
            r#type: "shortstring".to_string(),
        })],
    );

    // Create the message
    let mut message = IndexMap::new();
    message.insert(
        "publisher_name".to_string(),
        PrimitiveType::String(publisher_name.to_string()),
    );

    // Create TypedData
    let typed_data = TypedData::new(types, "Request", domain, message);

    Ok(typed_data)
}
