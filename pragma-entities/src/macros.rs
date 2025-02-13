/// Convert entry to database format
///
/// Arguments:
/// * `entry`: Entry to convert
/// * `signature`: Signature to use
///
/// Returns:
/// * `NewEntry`: New entry
#[macro_export]
macro_rules! convert_timestamp_to_datetime {
    ($timestamp:expr) => {{
        if $timestamp > (i64::MAX as u64).try_into().unwrap() {
            Err(EntryError::InvalidTimestamp(
                pragma_common::timestamp::TimestampRangeError::Other(format!(
                    "Timestamp {} is too large",
                    $timestamp
                )),
            ))
        } else if $timestamp.to_string().len() >= 13 {
            #[allow(clippy::cast_possible_wrap)]
            chrono::DateTime::<chrono::Utc>::from_timestamp_millis($timestamp as i64)
                .map(|dt| dt.naive_utc())
                .ok_or_else(|| {
                    EntryError::InvalidTimestamp(
                        pragma_common::timestamp::TimestampRangeError::Other(format!(
                            "Could not convert {} to DateTime (millis)",
                            $timestamp
                        )),
                    )
                })
        } else {
            #[allow(clippy::cast_possible_wrap)]
            chrono::DateTime::<chrono::Utc>::from_timestamp($timestamp as i64, 0)
                .map(|dt| dt.naive_utc())
                .ok_or_else(|| {
                    EntryError::InvalidTimestamp(
                        pragma_common::timestamp::TimestampRangeError::Other(format!(
                            "Could not convert {} to DateTime (seconds)",
                            $timestamp
                        )),
                    )
                })
        }
    }};
}

#[allow(clippy::cast_possible_wrap)]
#[cfg(test)]
mod tests {

    use crate::EntryError;
    use chrono::TimeZone;
    use chrono::Utc;

    #[test]
    fn test_current_timestamp() {
        let now = Utc::now().timestamp() as u64;
        let result = convert_timestamp_to_datetime!(now);
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.and_utc().timestamp(), now as i64);
    }

    #[test]
    fn test_current_timestamp_millis() {
        let now_millis = Utc::now().timestamp_millis() as u64;
        let result = convert_timestamp_to_datetime!(now_millis);
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.and_utc().timestamp_millis(), now_millis as i64);
    }

    #[test]
    fn test_specific_date() {
        // 2024-03-14 15:92:65 UTC (Pi Day!)
        let timestamp = 1_710_428_585_u64;
        let result = convert_timestamp_to_datetime!(timestamp);
        assert!(result.is_ok());
        let dt = result.unwrap();
        let expected = Utc
            .timestamp_opt(timestamp as i64, 0)
            .single()
            .expect("Invalid timestamp")
            .naive_utc();
        assert_eq!(dt, expected);
    }

    #[test]
    fn test_specific_date_millis() {
        // 2024-03-14 15:92:65.123 UTC
        let timestamp_millis = 1_710_428_585_123_u64;
        let result = convert_timestamp_to_datetime!(timestamp_millis);
        assert!(result.is_ok());
        let dt = result.unwrap();
        let expected = Utc
            .timestamp_millis_opt(timestamp_millis as i64)
            .single()
            .expect("Invalid timestamp")
            .naive_utc();
        assert_eq!(dt, expected);
    }

    #[test]
    fn test_boundary_timestamps() {
        // Test earliest valid timestamp (1970-01-01 00:00:00 UTC)
        let earliest = 0_u64;
        let result = convert_timestamp_to_datetime!(earliest);
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.and_utc().timestamp(), 0);

        // Test a very old timestamp (1970-01-02)
        let old = 86_400_u64;
        let result = convert_timestamp_to_datetime!(old);
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.and_utc().timestamp(), 86400);

        // Test a far future timestamp (2100-01-01)
        let future = 4_102_444_800_u64;
        let result = convert_timestamp_to_datetime!(future);
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.and_utc().timestamp(), future as i64);
    }

    #[test]
    fn test_invalid_timestamps() {
        // Test maximum u64 value
        let max_u64 = u64::MAX;
        let result = convert_timestamp_to_datetime!(max_u64);
        assert!(result.is_err());

        // Test value that's too large for i64 (milliseconds)
        let too_large_millis = (i64::MAX as u64) + 1;
        let result = convert_timestamp_to_datetime!(too_large_millis);
        assert!(result.is_err());
    }

    #[test]
    fn test_edge_cases() {
        // Test timestamp just below millisecond threshold (12 digits)
        let just_below_millis = 999_999_999_999_u64;
        let result = convert_timestamp_to_datetime!(just_below_millis);
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.and_utc().timestamp(), just_below_millis as i64);

        // Test timestamp just at millisecond threshold (13 digits)
        let just_millis = 1_000_000_000_000_u64;
        let result = convert_timestamp_to_datetime!(just_millis);
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.and_utc().timestamp_millis(), just_millis as i64);

        // Test a very recent timestamp
        let recent = (Utc::now().timestamp() - 1) as u64;
        let result = convert_timestamp_to_datetime!(recent);
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.and_utc().timestamp(), recent as i64);
    }

    #[test]
    fn test_real_world_scenarios() {
        // Test common exchange timestamp (e.g., Binance style)
        let binance_style = 1_710_428_585_123_u64; // millisecond precision
        let result = convert_timestamp_to_datetime!(binance_style);
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.and_utc().timestamp_millis(), binance_style as i64);

        // Test common blockchain timestamp (e.g., Ethereum block timestamp)
        let blockchain_style = 1_710_428_585_u64; // second precision
        let result = convert_timestamp_to_datetime!(blockchain_style);
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.and_utc().timestamp(), blockchain_style as i64);

        // Test Unix timestamp with recent date
        let unix_timestamp = Utc::now().timestamp() as u64;
        let result = convert_timestamp_to_datetime!(unix_timestamp);
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.and_utc().timestamp(), unix_timestamp as i64);
    }
}
