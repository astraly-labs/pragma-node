use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HexHash(pub String);

impl<'de> Deserialize<'de> for HexHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if !is_0x_prefixed_hex_string(&s) {
            return Err(serde::de::Error::custom("Invalid hex hash format"));
        }
        Ok(Self(s))
    }
}

// Helper function to check if a string is a valid 0x-prefixed hexadecimal string
fn is_0x_prefixed_hex_string(s: &str) -> bool {
    s.starts_with("0x") && s[2..].chars().all(|c| c.is_ascii_hexdigit())
}
