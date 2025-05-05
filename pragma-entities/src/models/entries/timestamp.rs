use serde::{Deserialize, Deserializer};
use std::ops::RangeInclusive;
use utoipa::ToSchema;

/// The number of seconds since the Unix epoch (00:00:00 UTC on 1 Jan 1970).
///
/// The timestamp is always positive, but represented as a signed integer
/// because that's the standard on Unix systems and allows easy subtraction
/// to compute durations.
pub type UnixTimestamp = i64;

/// Represents a range of timestamps
#[derive(Debug, Clone, ToSchema)]
#[schema(value_type = String)]
pub struct TimestampRange(pub RangeInclusive<UnixTimestamp>);

#[derive(Debug, thiserror::Error, ToSchema)]
pub enum TimestampRangeError {
    #[error("Start timestamp is after end timestamp")]
    StartAfterEnd,
    #[error("End timestamp is in the future")]
    EndInFuture,
    #[error("Start timestamp equals end timestamp")]
    StartEqualsEnd,
    #[error("Could not convert timestamp to DateTime")]
    ConversionError,
    #[error("Other error: {0}")]
    Other(String),
}

#[derive(Debug, thiserror::Error, ToSchema)]
pub enum TimestampError {
    #[error("Timestamp range error: {0}")]
    RangeError(#[from] TimestampRangeError),
    #[error("Could not convert unsigned timestamp to datetime: {0}")]
    ToDatetimeErrorU64(u64),
    #[error("Could not convert signed timestamp to datetime: {0}")]
    ToDatetimeErrorI64(i64),
    #[error("Timestamp error: {0}")]
    Other(String),
}

impl TimestampRange {
    pub fn assert_time_is_valid(self) -> Result<Self, TimestampRangeError> {
        let now = chrono::Utc::now().timestamp();
        let range = &self.0;

        if range.start() > range.end() {
            return Err(TimestampRangeError::StartAfterEnd);
        }
        if *range.end() > now {
            return Err(TimestampRangeError::EndInFuture);
        }
        if *range.start() == *range.end() {
            return Err(TimestampRangeError::StartEqualsEnd);
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
        let s = s.replace(' ', "");
        let (start, end) = s
            .split_once(',')
            .ok_or_else(|| serde::de::Error::custom("Expected format: start,end"))?;
        let start = start.parse().map_err(serde::de::Error::custom)?;
        let end = end.parse().map_err(serde::de::Error::custom)?;
        Ok(Self(start..=end))
    }
}
