use pragma_entities::EntryError;
use serde::{Deserialize, Deserializer};
use std::ops::RangeInclusive;
use utoipa::ToSchema;

/// The number of seconds since the Unix epoch (00:00:00 UTC on 1 Jan 1970). The timestamp is
/// always positive, but represented as a signed integer because that's the standard on Unix
/// systems and allows easy subtraction to compute durations.
pub type UnixTimestamp = i64;

/// Represents a range of timestamps
#[derive(Debug, Clone, ToSchema)]
pub struct TimestampRange(pub RangeInclusive<UnixTimestamp>);

impl TimestampRange {
    pub fn assert_time_is_valid(self) -> Result<Self, EntryError> {
        let now = chrono::Utc::now().timestamp();
        let range = &self.0;

        if range.start() > range.end() {
            return Err(EntryError::InvalidTimestamp(
                "Range timestamp first date is greater than the second date.".into(),
            ));
        }
        if *range.end() > now {
            return Err(EntryError::InvalidTimestamp(
                "Range timestamp end is in the future.".into(),
            ));
        }
        if *range.start() == *range.end() {
            return Err(EntryError::InvalidTimestamp(
                "Range timestamp start and end have the same value.".into(),
            ));
        }

        Ok(self)
    }
}

impl<'de> Deserialize<'de> for TimestampRange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;

        let (start, end) = s
            .split_once(',')
            .ok_or_else(|| serde::de::Error::custom("Expected format: start,end"))?;
        let start = start.parse().map_err(serde::de::Error::custom)?;
        let end = end.parse().map_err(serde::de::Error::custom)?;
        Ok(TimestampRange(start..=end))
    }
}
