//! Contains utils from the pragma-node repository.
//! Since we can't import them here, we recreated them here.
use pragma_common::types::Interval;

pub const fn get_interval_specifier(interval: Interval, is_twap: bool) -> &'static str {
    if is_twap {
        match interval {
            Interval::OneMinute => "1_min",
            Interval::FiveMinutes => "5_min",
            Interval::FifteenMinutes => "15_min",
            Interval::OneHour => "1_hour",
            Interval::TwoHours => "2_hours",
            Interval::OneDay => "1_day",
            _ => panic!("unsupported interval"),
        }
    } else {
        match interval {
            Interval::OneHundredMillisecond => "100_ms",
            Interval::OneSecond => "1_s",
            Interval::FiveSeconds => "5_s",
            Interval::TenSeconds => "10_s",
            Interval::OneMinute => "1_min",
            Interval::FiveMinutes => "5_min",
            Interval::FifteenMinutes => "15_min",
            Interval::OneHour => "1_h",
            Interval::TwoHours => "2_h",
            Interval::OneDay => "1_day",
            Interval::OneWeek => "1_week",
        }
    }
}

pub const fn get_window_size(interval: Interval) -> i64 {
    match interval {
        Interval::OneHundredMillisecond => 1, // 1 second window
        Interval::OneSecond => 10,            // 10 seconds window
        Interval::FiveSeconds => 30,          // 30 seconds window
        Interval::TenSeconds => 60,           // 60 seconds window
        Interval::OneMinute => 300,           // 5 minutes window
        Interval::FiveMinutes => 900,         // 15 minutes window
        Interval::FifteenMinutes => 1800,     // 30 minutes window
        Interval::OneHour => 7200,            // 2 hours window
        Interval::TwoHours => 14400,          // 4 hours window
        Interval::OneDay => 86400,            // 24 hours window
        Interval::OneWeek => 604800,          // 1 week window
    }
}
