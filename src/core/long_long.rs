//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! 64-bit integer helper (`wxLongLong`).

use std::fmt;

/// Signed 64-bit integer wrapper (`wxLongLong`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct LongLong(pub i64);

impl LongLong {
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    pub fn value(self) -> i64 {
        self.0
    }

    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }

    pub fn to_string_decimal(self) -> String {
        self.0.to_string()
    }
}

impl fmt::Display for LongLong {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Add for LongLong {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl std::ops::Sub for LongLong {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl From<i64> for LongLong {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl From<LongLong> for i64 {
    fn from(value: LongLong) -> Self {
        value.0
    }
}
