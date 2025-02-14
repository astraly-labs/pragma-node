// Reference:
// https://github.com/xJonathanLEI/starknet-rs/blob/master/starknet-core/src/types/codegen.rs#L71
use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;
use strum::{Display, EnumString};
use utoipa::ToSchema;

/// Block tag.
///
/// A tag specifying a dynamic reference to a block.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    ToSchema,
)]
#[strum(serialize_all = "lowercase")]
pub enum BlockTag {
    Latest,
    #[default]
    Pending,
}

/// Block identifier in the form of hash, number or tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, ToSchema)]
pub enum BlockId {
    #[strum(serialize = "{0}")]
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
        u64::from_str(&value).map_or_else(
            |_| {
                BlockTag::from_str(&value.to_lowercase()).map_or_else(
                    |_| {
                        Err(serde::de::Error::custom(format!(
                            "Invalid BlockId: {value}"
                        )))
                    },
                    |tag| Ok(Self::Tag(tag)),
                )
            },
            |num| Ok(Self::Number(num)),
        )
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
