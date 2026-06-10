//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Date and time spans (`wxDateSpan`, `wxTimeSpan`).

/// Relative calendar offset (`wxDateSpan`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateSpan {
    pub years: i32,
    pub months: i32,
    pub days: i32,
}

impl DateSpan {
    pub const fn days(days: i32) -> Self {
        Self {
            years: 0,
            months: 0,
            days,
        }
    }

    pub const fn months(months: i32) -> Self {
        Self {
            years: 0,
            months,
            days: 0,
        }
    }

    pub const fn years(years: i32) -> Self {
        Self {
            years,
            months: 0,
            days: 0,
        }
    }
}

/// Relative time offset (`wxTimeSpan`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeSpan {
    pub seconds: i64,
}

impl TimeSpan {
    pub const fn seconds(seconds: i64) -> Self {
        Self { seconds }
    }

    pub const fn minutes(minutes: i64) -> Self {
        Self {
            seconds: minutes * 60,
        }
    }

    pub const fn hours(hours: i64) -> Self {
        Self {
            seconds: hours * 3600,
        }
    }

    pub fn as_seconds(self) -> i64 {
        self.seconds
    }
}
