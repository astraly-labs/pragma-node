use pragma_common::types::{AggregationMode, DataType, Interval};
use pragma_entities::InfraError;

// SQL statement used to filter the expiration timestamp for future entries
pub fn get_expiration_timestamp_filter(
    data_type: DataType,
    expiry: &str,
) -> Result<String, InfraError> {
    match data_type {
        DataType::SpotEntry => Ok(String::default()),
        DataType::PerpEntry => {
            Ok(String::from("AND\n\t\texpiration_timestamp is null"))
        }
        DataType::FutureEntry if !expiry.is_empty() => {
            Ok(format!("AND\n\texpiration_timestamp = '{expiry}'"))
        }
        _ => Err(InfraError::InternalServerError),
    }
}

// Retrieve the timescale table based on the network and data type.
pub const fn get_table_suffix(data_type: DataType) -> Result<&'static str, InfraError> {
    match data_type {
        DataType::SpotEntry => Ok(""),
        DataType::FutureEntry => Ok("_future"),
        DataType::PerpEntry => Ok("_future"),
    }
}

// Retrieve the timeframe specifier based on the interval and aggregation mode.
pub const fn get_interval_specifier(
    interval: Interval,
    is_twap: bool,
) -> Result<&'static str, InfraError> {
    if is_twap {
        match interval {
            Interval::OneHour => Ok("1_hour"),
            Interval::TwoHours => Ok("2_hours"),
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
            Interval::OneMinute => Ok("1_min"),
            Interval::FifteenMinutes => Ok("15_min"),
            Interval::OneHour => Ok("1_h"),
            Interval::TwoHours => Ok("2_h"),
            Interval::OneDay => Ok("1_day"),
            Interval::OneWeek => Ok("1_week"),
        }
    }
}
