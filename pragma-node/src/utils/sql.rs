use pragma_common::types::{AggregationMode, DataType, Interval};
use pragma_entities::InfraError;

// Retrieve the timescale table based on the network and data type.
pub const fn get_table_suffix(data_type: DataType) -> Result<&'static str, InfraError> {
    match data_type {
        DataType::SpotEntry => Ok("spot"),
        DataType::PerpEntry => Ok("perp"),
        DataType::FutureEntry => Ok("future"),
    }
}

// Retrieve the timeframe specifier based on the interval and aggregation mode.
pub const fn get_interval_specifier(
    interval: Interval,
    is_twap: bool,
) -> Result<&'static str, InfraError> {
    if is_twap {
        match interval {
            Interval::OneMinute => Ok("1_min"),
            Interval::FiveMinutes => Ok("5_min"),
            Interval::FifteenMinutes => Ok("15_min"),
            Interval::OneHour => Ok("1_h"),
            Interval::TwoHours => Ok("2_h"),
            Interval::OneDay => Ok("1_day"),
            _ => Err(InfraError::UnsupportedInterval(
                interval,
                AggregationMode::Twap,
            )),
        }
    } else {
        match interval {
            Interval::OneHundredMillisecond => Ok("100_ms"),
            Interval::OneSecond => Ok("1_s"),
            Interval::FiveSeconds => Ok("5_s"),
            Interval::TenSeconds => Ok("10_s"),
            Interval::OneMinute => Ok("1_min"),
            Interval::FifteenMinutes => Ok("15_min"),
            Interval::OneHour => Ok("1_h"),
            Interval::TwoHours => Ok("2_h"),
            Interval::OneDay => Ok("1_day"),
            Interval::OneWeek => Ok("1_week"),
            Interval::FiveMinutes => Err(InfraError::UnsupportedInterval(
                interval,
                AggregationMode::Median,
            )),
        }
    }
}
