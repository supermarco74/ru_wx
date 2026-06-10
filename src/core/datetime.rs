//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Date and time value (`wxDateTime`).

use std::fmt;

use crate::core::datetime_span::{DateSpan, TimeSpan};

/// Calendar date with optional time-of-day (`wxDateTime`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DateTime {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl DateTime {
    pub const fn new(year: i32, month: u32, day: u32) -> Self {
        Self {
            year,
            month,
            day,
            hour: 0,
            minute: 0,
            second: 0,
        }
    }

    pub const fn with_time(mut self, hour: u32, minute: u32, second: u32) -> Self {
        self.hour = hour;
        self.minute = minute;
        self.second = second;
        self
    }

    pub fn today() -> Self {
        Self::from_ymd(1970, 1, 1)
    }

    pub fn from_ymd(year: i32, month: u32, day: u32) -> Self {
        Self::new(year, month, day)
    }

    pub fn add_days(self, days: i32) -> Self {
        self + DateSpan::days(days)
    }

    pub fn add_seconds(self, seconds: i64) -> Self {
        self + TimeSpan::seconds(seconds)
    }

    pub fn format_iso(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_iso())
    }
}

impl std::ops::Add<DateSpan> for DateTime {
    type Output = Self;

    fn add(self, span: DateSpan) -> Self {
        let mut dt = self;
        dt.day = dt.day.saturating_add_signed(span.days);
        dt.month = dt.month.saturating_add_signed(span.months);
        dt.year = dt.year.saturating_add(span.years);
        dt
    }
}

impl std::ops::Add<TimeSpan> for DateTime {
    type Output = Self;

    fn add(self, span: TimeSpan) -> Self {
        let total = self.hour as i64 * 3600
            + self.minute as i64 * 60
            + self.second as i64
            + span.seconds;
        let days = total.div_euclid(86_400);
        let rem = total.rem_euclid(86_400);
        let mut dt = self.add_days(days as i32);
        dt.hour = (rem / 3600) as u32;
        dt.minute = ((rem % 3600) / 60) as u32;
        dt.second = (rem % 60) as u32;
        dt
    }
}
