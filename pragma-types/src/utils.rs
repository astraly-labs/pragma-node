use std::str::FromStr;
use serde::{Deserialize as _, Deserializer};
use starknet::core::types::Felt;

/// Deserializes a vector of Felt from a JSON array of strings.
pub fn felt_from_decimal<'de, D>(deserializer: D) -> Result<Vec<Felt>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Vec<String> = Vec::deserialize(deserializer)?;
    Ok(s.iter().map(|s| Felt::from_dec_str(s).unwrap()).collect())
}

/// Deserializes a u128 from a JSON string or number.
pub fn flexible_u128<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    // Try deserializing to Value first to handle both formats
    let value = serde_json::Value::deserialize(deserializer)?;

    match value {
        serde_json::Value::String(s) => u128::from_str(&s).map_err(D::Error::custom),
        serde_json::Value::Number(n) => {
            let s = n.to_string();
            u128::from_str(&s).map_err(D::Error::custom)
        }
        _ => Err(D::Error::custom("expected string or number")),
    }
}