use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Number;
use std::fmt;
use utoipa::ToSchema;

use crate::typed_data::{Domain, Field, PrimitiveType, SimpleField, TypedData};
use crate::types::utils::flexible_u128;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct BaseEntry {
    pub timestamp: u64,
    pub source: String,
    pub publisher: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(tag = "type", content = "data")]
pub enum MarketEntry {
    #[serde(rename = "spot")]
    Spot(Entry),
    #[serde(rename = "perp")]
    Perp(PerpEntry),
}

impl EntryTrait for MarketEntry {
    fn base(&self) -> &BaseEntry {
        match self {
            Self::Spot(entry) => entry.base(),
            Self::Perp(entry) => entry.base(),
        }
    }

    fn pair_id(&self) -> &String {
        match self {
            Self::Spot(entry) => entry.pair_id(),
            Self::Perp(entry) => entry.pair_id(),
        }
    }

    fn price(&self) -> u128 {
        match self {
            Self::Spot(entry) => entry.price(),
            Self::Perp(entry) => entry.price(),
        }
    }

    fn volume(&self) -> u128 {
        match self {
            Self::Spot(entry) => entry.volume(),
            Self::Perp(entry) => entry.volume(),
        }
    }

    fn expiration_timestamp(&self) -> Option<u64> {
        match self {
            Self::Spot(entry) => entry.expiration_timestamp(),
            Self::Perp(entry) => entry.expiration_timestamp(),
        }
    }
}

impl fmt::Display for MarketEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spot(entry) => write!(f, "spot: {entry}"),
            Self::Perp(entry) => write!(f, "perp: {entry}"),
        }
    }
}

pub trait EntryTrait {
    fn base(&self) -> &BaseEntry;
    fn pair_id(&self) -> &String;
    fn price(&self) -> u128;
    fn volume(&self) -> u128;
    fn expiration_timestamp(&self) -> Option<u64> {
        None
    }
}

// Entry = SpotEntry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct Entry {
    pub base: BaseEntry,
    pub pair_id: String,
    #[serde(deserialize_with = "flexible_u128")]
    pub price: u128,
    #[serde(deserialize_with = "flexible_u128")]
    pub volume: u128,
}

impl EntryTrait for Entry {
    fn base(&self) -> &BaseEntry {
        &self.base
    }

    fn pair_id(&self) -> &String {
        &self.pair_id
    }

    fn price(&self) -> u128 {
        self.price
    }

    fn volume(&self) -> u128 {
        self.volume
    }
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SPOT[{}] {} @ {} (vol: {}) from {}/{}",
            self.pair_id,
            self.price,
            self.base.timestamp,
            self.volume,
            self.base.source,
            self.base.publisher
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct PerpEntry {
    pub base: BaseEntry,
    pub pair_id: String,
    #[serde(deserialize_with = "flexible_u128")]
    pub price: u128,
    #[serde(deserialize_with = "flexible_u128")]
    pub volume: u128,
}

impl EntryTrait for PerpEntry {
    fn base(&self) -> &BaseEntry {
        &self.base
    }

    fn pair_id(&self) -> &String {
        &self.pair_id
    }

    fn price(&self) -> u128 {
        self.price
    }

    fn volume(&self) -> u128 {
        self.volume
    }

    fn expiration_timestamp(&self) -> Option<u64> {
        Some(0)
    }
}

impl fmt::Display for PerpEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PERP[{}] {} @ {} (vol: {}) from {}/{}",
            self.pair_id,
            self.price,
            self.base.timestamp,
            self.volume,
            self.base.source,
            self.base.publisher
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct FutureEntry {
    pub base: BaseEntry,
    pub pair_id: String,
    #[serde(deserialize_with = "flexible_u128")]
    pub price: u128,
    #[serde(deserialize_with = "flexible_u128")]
    pub volume: u128,
    // in milliseconds
    pub expiration_timestamp: u64,
}

impl EntryTrait for FutureEntry {
    fn base(&self) -> &BaseEntry {
        &self.base
    }

    fn pair_id(&self) -> &String {
        &self.pair_id
    }

    fn price(&self) -> u128 {
        self.price
    }

    fn volume(&self) -> u128 {
        self.volume
    }

    fn expiration_timestamp(&self) -> Option<u64> {
        Some(self.expiration_timestamp)
    }
}

impl fmt::Display for FutureEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FUTURE[{}] {} @ {} (vol: {}, exp: {}) from {}/{}",
            self.pair_id,
            self.price,
            self.base.timestamp,
            self.volume,
            self.expiration_timestamp,
            self.base.source,
            self.base.publisher
        )
    }
}

#[allow(clippy::too_many_lines)]
pub fn build_publish_message<E>(entries: &[E]) -> TypedData
where
    E: EntryTrait + Serialize + for<'a> Deserialize<'a>,
{
    let mut is_future = false;

    // Construct the raw entries
    let raw_entries: Vec<PrimitiveType> = entries
        .iter()
        .map(|entry| {
            let mut entry_map = IndexMap::new();
            let base = entry.base();

            // Add base fields
            let mut base_map = IndexMap::new();
            base_map.insert(
                "publisher".to_string(),
                PrimitiveType::String(base.publisher.clone()),
            );
            base_map.insert(
                "source".to_string(),
                PrimitiveType::String(base.source.clone()),
            );
            base_map.insert(
                "timestamp".to_string(),
                PrimitiveType::String(base.timestamp.to_string()),
            );

            entry_map.insert("base".to_string(), PrimitiveType::Object(base_map));
            entry_map.insert(
                "pair_id".to_string(),
                PrimitiveType::String(entry.pair_id().to_string()),
            );
            entry_map.insert(
                "price".to_string(),
                PrimitiveType::Number(Number::from(entry.price())),
            );
            entry_map.insert(
                "volume".to_string(),
                PrimitiveType::Number(Number::from(entry.volume())),
            );

            // Handle optional expiration timestamp
            if let Some(expiration) = entry.expiration_timestamp() {
                is_future = true;
                entry_map.insert(
                    "expiration_timestamp".to_string(),
                    PrimitiveType::String(expiration.to_string()),
                );
            }

            PrimitiveType::Object(entry_map)
        })
        .collect();

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

    // Define "Entry" type
    let mut entry_fields = vec![
        Field::SimpleType(SimpleField {
            name: "base".to_string(),
            r#type: "Base".to_string(),
        }),
        Field::SimpleType(SimpleField {
            name: "pair_id".to_string(),
            r#type: "shortstring".to_string(),
        }),
        Field::SimpleType(SimpleField {
            name: "price".to_string(),
            r#type: "u128".to_string(),
        }),
        Field::SimpleType(SimpleField {
            name: "volume".to_string(),
            r#type: "u128".to_string(),
        }),
    ];

    // Include "expiration_timestamp" if necessary
    if is_future {
        entry_fields.push(Field::SimpleType(SimpleField {
            name: "expiration_timestamp".to_string(),
            r#type: "timestamp".to_string(),
        }));
    }

    types.insert("Entry".to_string(), entry_fields);

    // Define "Base" type
    types.insert(
        "Base".to_string(),
        vec![
            Field::SimpleType(SimpleField {
                name: "publisher".to_string(),
                r#type: "shortstring".to_string(),
            }),
            Field::SimpleType(SimpleField {
                name: "source".to_string(),
                r#type: "shortstring".to_string(),
            }),
            Field::SimpleType(SimpleField {
                name: "timestamp".to_string(),
                r#type: "timestamp".to_string(),
            }),
        ],
    );

    // **Add the missing "Request" type**
    types.insert(
        "Request".to_string(),
        vec![
            Field::SimpleType(SimpleField {
                name: "action".to_string(),
                r#type: "shortstring".to_string(),
            }),
            Field::SimpleType(SimpleField {
                name: "entries".to_string(),
                r#type: "Entry*".to_string(),
            }),
        ],
    );

    // Create the message
    let mut message = IndexMap::new();
    message.insert(
        "action".to_string(),
        PrimitiveType::String("Publish".to_string()),
    );
    message.insert("entries".to_string(), PrimitiveType::Array(raw_entries));

    TypedData::new(types, "Request", domain, message)
}
