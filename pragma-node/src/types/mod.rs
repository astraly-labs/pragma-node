use serde::{Deserialize, Deserializer};
use std::ops::RangeInclusive;

pub mod entries;
pub mod pricer;
pub mod ws;

/// The number of seconds since the Unix epoch (00:00:00 UTC on 1 Jan 1970). The timestamp is
/// always positive, but represented as a signed integer because that's the standard on Unix
/// systems and allows easy subtraction to compute durations.
pub type UnixTimestamp = i64;

#[derive(Debug, Clone)]
pub enum TimestampParam {
    Single(u64),
    Range(RangeInclusive<u64>),
}

impl From<u64> for TimestampParam {
    fn from(ts: u64) -> Self {
        TimestampParam::Single(ts)
    }
}

impl<'de> Deserialize<'de> for TimestampParam {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        if let Some((start, end)) = s.split_once(',') {
            let start = start.parse().map_err(serde::de::Error::custom)?;
            let end = end.parse().map_err(serde::de::Error::custom)?;
            Ok(TimestampParam::Range(start..=end))
        } else {
            let ts = s.parse().map_err(serde::de::Error::custom)?;
            Ok(TimestampParam::Single(ts))
        }
    }
}
