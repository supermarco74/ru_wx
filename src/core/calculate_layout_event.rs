//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Layout calculation event (`wxCalculateLayoutEvent`).

use crate::core::geometry::{Point, Size};

/// Sizer requests final child layout (`wxCalculateLayoutEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalculateLayoutEvent {
    pub origin: Point,
    pub size: Size,
}

impl CalculateLayoutEvent {
    pub const fn new(origin: Point, size: Size) -> Self {
        Self { origin, size }
    }
}
