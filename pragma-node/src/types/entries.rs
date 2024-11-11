use pragma_entities::EntryError;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::utils::TypedData;

#[derive(Debug, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct BaseEntry {
    pub timestamp: u64,
    pub source: String,
    pub publisher: String,
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
#[derive(Debug, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct Entry {
    pub base: BaseEntry,
    pub pair_id: String,
    pub price: u128,
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

#[derive(Debug, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct PerpEntry {
    pub base: BaseEntry,
    pub pair_id: String,
    pub price: u128,
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

#[derive(Debug, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct FutureEntry {
    pub base: BaseEntry,
    pub pair_id: String,
    pub price: u128,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct PublishMessage<E: EntryTrait + Serialize> {
    pub action: String,
    pub entries: Vec<E>,
}

// TODO: Remove this legacy handling while every publishers are on the 2.0 version.
pub fn build_publish_message<E>(
    entries: &[E],
    is_legacy: Option<bool>,
) -> Result<TypedData<PublishMessage<E>>, EntryError>
where
    E: EntryTrait + Serialize + for<'a> Deserialize<'a>,
{
    // TODO(akhercha): ugly, refine
    let mut is_future = false;
    let is_legacy = is_legacy.unwrap_or(false);

    // Construct the raw string with placeholders for the entries
    let raw_entries: Vec<_> = entries
        .iter()
        .map(|entry| {
            let base = entry.base();
            let pair_id = entry.pair_id();
            let price = entry.price();
            let volume = entry.volume();
            let expiration_timestamp = entry.expiration_timestamp();

            let mut entry_map = serde_json::json!({
                "base": {
                    "publisher": base.publisher,
                    "source": base.source,
                    "timestamp": base.timestamp
                },
                "pair_id": pair_id,
                "price": price,
                "volume": volume
            });

            if let Some(expiration) = expiration_timestamp {
                is_future = true;
                entry_map["expiration_timestamp"] = serde_json::json!(expiration);
            }

            entry_map
        })
        .collect::<Vec<_>>();

    // We recently updated our Pragma-SDK. This included a breaking change for how we
    // sign the entries before publishing them.
    // We want to support our publishers who are still on the older version and
    // encourage them to upgrade before removing this legacy code. Until then,
    // we support both methods.
    // TODO: Remove this legacy handling while every publishers are on the 2.0 version.
    let mut raw_message_json = if is_legacy {
        serde_json::json!({
            "domain": {
                "name": "Pragma",
                "version": "1"
            },
            "primaryType": "Request",
            "message": {
                "action": "Publish",
                "entries": raw_entries
            },
            "types": {
                "StarknetDomain": [
                    {"name": "name", "type": "felt"},
                    {"name": "version", "type": "felt"}
                ],
                "Request": [
                    {"name": "action", "type": "felt"},
                    {"name": "entries", "type": "Entry*"}
                ],
                "Entry": [
                    {"name": "base", "type": "Base"},
                    {"name": "pair_id", "type": "felt"},
                    {"name": "price", "type": "felt"},
                    {"name": "volume", "type": "felt"},
                ],
                "Base": [
                    {"name": "publisher", "type": "felt"},
                    {"name": "source", "type": "felt"},
                    {"name": "timestamp", "type": "felt"}
                ]
            }
        })
    } else {
        serde_json::json!({
            "domain": {
                "name": "Pragma",
                "version": "1",
                "chainId": "1",
                "revision": "0"
            },
            "primaryType": "Request",
            "message": {
                "action": "Publish",
                "entries": raw_entries
            },
            "types": {
                "StarknetDomain": [
                    {"name": "name", "type": "felt"},
                    {"name": "version", "type": "felt"},
                    {"name": "chainId", "type": "felt"},
                    {"name": "revision", "type": "felt"}
                ],
                "Request": [
                    {"name": "action", "type": "felt"},
                    {"name": "entries", "type": "Entry*"}
                ],
                "Entry": [
                    {"name": "base", "type": "Base"},
                    {"name": "pair_id", "type": "felt"},
                    {"name": "price", "type": "felt"},
                    {"name": "volume", "type": "felt"},
                ],
                "Base": [
                    {"name": "publisher", "type": "felt"},
                    {"name": "source", "type": "felt"},
                    {"name": "timestamp", "type": "felt"}
                ]
            }
        })
    };

    // Add the expiration timestamp for the future entries
    if is_future {
        let types = raw_message_json["types"].as_object_mut().unwrap();
        let entry = types["Entry"].as_array_mut().unwrap();
        entry.push(serde_json::json!({"name": "expiration_timestamp", "type": "felt"}));
    }

    serde_json::from_value(raw_message_json).map_err(|e| EntryError::BuildPublish(e.to_string()))
}
