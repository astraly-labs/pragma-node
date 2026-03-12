//! Contains utils from the pragma-node repository.
//! Since we can't import them here, we recreated them here.
use pragma_common::Interval;

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
