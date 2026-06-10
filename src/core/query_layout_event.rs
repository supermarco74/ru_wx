//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Layout query event (`wxQueryLayoutEvent`).

use crate::core::geometry::Size;

/// Window asks children for best size (`wxQueryLayoutEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryLayoutEvent {
    pub proposed: Size,
}

impl QueryLayoutEvent {
    pub const fn new(proposed: Size) -> Self {
        Self { proposed }
    }
}
