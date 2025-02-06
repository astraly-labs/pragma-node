use indexmap::IndexMap;
use pragma_entities::models::entry_error::SigningError;
use serde::{Deserialize, Serialize};
use starknet::core::types::Felt;
use utoipa::ToSchema;

use crate::typed_data::{Domain, Field, PrimitiveType, SimpleField, TypedData};
use crate::utils::felt_from_decimal;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginMessage {
    #[schema(value_type = Vec<String>)]
    #[serde(deserialize_with = "felt_from_decimal")]
    pub signature: Vec<Felt>,
    pub publisher_name: String,
    pub expiration_timestamp: u64,
}

pub fn build_login_message(
    publisher_name: &str,
    expiration_timestamp: u64,
) -> Result<TypedData, SigningError> {
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
        vec![
            Field::SimpleType(SimpleField {
                name: "publisher_name".to_string(),
                r#type: "shortstring".to_string(),
            }),
            Field::SimpleType(SimpleField {
                name: "expiration_timestamp".to_string(),
                r#type: "timestamp".to_string(),
            }),
        ],
    );

    // Create the message
    let mut message = IndexMap::new();
    message.insert(
        "publisher_name".to_string(),
        PrimitiveType::String(publisher_name.to_string()),
    );
    message.insert(
        "expiration_timestamp".to_string(),
        PrimitiveType::Number(expiration_timestamp.into()),
    );

    // Create TypedData
    let typed_data = TypedData::new(types, "Request", domain, message);

    Ok(typed_data)
}
