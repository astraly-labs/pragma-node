// Reference:
// https://github.com/xJonathanLEI/starknet-rs/blob/master/starknet-core/src/types/codegen.rs#L71
use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;
use strum::{Display, EnumString};

/// Block tag.
///
/// A tag specifying a dynamic reference to a block.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, EnumString,
)]
#[strum(serialize_all = "lowercase")]
pub enum BlockTag {
    #[default]
    Latest,
}

/// Block identifier in the form of hash, number or tag.
/// Block identifier in the form of hash, number or tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum BlockId {
    #[strum(serialize = "latest")]
    Tag(BlockTag),
    #[strum(serialize = "{0}")]
    Number(u64),
}

impl<'de> Deserialize<'de> for BlockId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if let Ok(num) = u64::from_str(&value) {
            Ok(BlockId::Number(num))
        } else if let Ok(tag) = BlockTag::from_str(&value.to_lowercase()) {
            Ok(BlockId::Tag(tag))
        } else {
            Err(serde::de::Error::custom(format!(
                "Invalid BlockId: {}",
                value
            )))
        }
    }
}

impl Serialize for BlockId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
