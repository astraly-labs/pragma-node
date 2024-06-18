use crate::types::UnixTimestamp;

/// Example value for a unix timestamp
pub fn timestamp_example() -> UnixTimestamp {
    const STATIC_UNIX_TIMESTAMP: UnixTimestamp = 1717632000; // Thursday, 6 June 2024 00:00:00 GMT
    STATIC_UNIX_TIMESTAMP
}
