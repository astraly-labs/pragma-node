use chrono::{NaiveDateTime, Timelike};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug)]
pub enum ConversionError {
    FailedSerialization,
    InvalidDateTime,
    BigDecimalConversion,
    FeltConversion,
    U128Conversion,
}

#[derive(Default, Debug, Serialize, Deserialize, ToSchema, Clone, Copy)]
pub enum AggregationMode {
    #[serde(rename = "median")]
    #[default]
    Median,
    #[serde(rename = "mean")]
    Mean,
    #[serde(rename = "twap")]
    Twap,
}

#[derive(Default, Debug, Serialize, Deserialize, ToSchema, Clone, Copy)]
pub enum Network {
    #[serde(rename = "testnet")]
    #[default]
    Testnet,
    #[serde(rename = "mainnet")]
    Mainnet,
}

#[derive(Default, Debug, Deserialize, ToSchema, Clone, Copy)]
pub enum DataType {
    #[serde(rename = "spot_entry")]
    #[default]
    SpotEntry,
    #[serde(rename = "perp_entry")]
    PerpEntry,
    #[serde(rename = "future_entry")]
    FutureEntry,
}

// Supported Aggregation Intervals
#[derive(Default, Debug, Serialize, Deserialize, ToSchema, Clone, Copy)]
pub enum Interval {
    #[serde(rename = "1min")]
    #[default]
    OneMinute,
    #[serde(rename = "15min")]
    FifteenMinutes,
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "2h")]
    TwoHours,
}

impl Interval {
    pub fn to_minutes(&self) -> i64 {
        match self {
            Interval::OneMinute => 1,
            Interval::FifteenMinutes => 15,
            Interval::OneHour => 60,
            Interval::TwoHours => 120,
        }
    }

    pub fn to_seconds(&self) -> i64 {
        self.to_minutes() * 60
    }

    /// Align a datetime to the nearest interval boundary.
    ///
    /// This function ensures that the given datetime is aligned to the self interval.
    /// For example, if the interval is 15 minutes, a datetime like 20:17 will be
    /// adjusted to 20:15, so that it falls on the boundary of the interval.
    pub fn align_timestamp(&self, dt: NaiveDateTime) -> NaiveDateTime {
        let interval_minutes = self.to_minutes();
        let dt_minutes = dt.minute() as i64;
        let total_minutes = dt.hour() as i64 * 60 + dt_minutes;

        let aligned_total_minutes = (total_minutes / interval_minutes) * interval_minutes;
        let aligned_hours = aligned_total_minutes / 60;
        let aligned_minutes = aligned_total_minutes % 60;

        dt.with_minute(aligned_minutes as u32)
            .unwrap()
            .with_hour(aligned_hours as u32)
            .unwrap()
            .with_second(0)
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::Interval;
    use chrono::{DateTime, NaiveDateTime};

    #[test]
    fn test_align_timestamp() {
        let test_inputs = [
            (
                Interval::OneMinute,
                vec![
                    ("2021-01-01T00:00:00Z", "2021-01-01 00:00:00"),
                    ("2021-01-01T00:00:30Z", "2021-01-01 00:00:00"),
                    ("2021-01-01T00:01:00Z", "2021-01-01 00:01:00"),
                    ("2021-01-01T00:01:30Z", "2021-01-01 00:01:00"),
                ],
            ),
            (
                Interval::FifteenMinutes,
                vec![
                    ("2021-01-01T00:00:00Z", "2021-01-01 00:00:00"),
                    ("2021-01-01T00:00:30Z", "2021-01-01 00:00:00"),
                    ("2021-01-01T00:01:30Z", "2021-01-01 00:00:00"),
                    ("2021-01-01T00:00:30Z", "2021-01-01 00:00:00"),
                    ("2021-01-01T00:15:00Z", "2021-01-01 00:15:00"),
                    ("2021-01-01T00:22:30Z", "2021-01-01 00:15:00"),
                ],
            ),
            (
                Interval::OneHour,
                vec![
                    ("2021-01-01T00:00:00Z", "2021-01-01 00:00:00"),
                    ("2021-01-01T00:30:00Z", "2021-01-01 00:00:00"),
                    ("2021-01-01T01:00:00Z", "2021-01-01 01:00:00"),
                    ("2021-01-01T01:30:00Z", "2021-01-01 01:00:00"),
                ],
            ),
            (
                Interval::TwoHours,
                vec![
                    ("2021-01-01T00:00:00Z", "2021-01-01 00:00:00"),
                    ("2021-01-01T01:30:00Z", "2021-01-01 00:00:00"),
                    ("2021-01-01T02:00:00Z", "2021-01-01 02:00:00"),
                    ("2021-01-01T02:30:00Z", "2021-01-01 02:00:00"),
                ],
            ),
        ];
        for (interval, test_case) in test_inputs.iter() {
            for (input, expected) in test_case.iter() {
                let dt: NaiveDateTime = DateTime::parse_from_rfc3339(input).unwrap().naive_utc();
                let aligned_dt = interval.align_timestamp(dt);
                assert_eq!(aligned_dt.to_string(), *expected);
            }
        }
    }
}
