//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Unsigned 64-bit integer helper (`wxULongLong`).

use std::fmt;

/// Unsigned 64-bit integer wrapper (`wxULongLong`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ULongLong(pub u64);

impl ULongLong {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn value(self) -> u64 {
        self.0
    }

    pub fn to_string_decimal(self) -> String {
        self.0.to_string()
    }
}

impl fmt::Display for ULongLong {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Add for ULongLong {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl From<u64> for ULongLong {
    fn from(value: u64) -> Self {
        Self(value)
    }
}
