//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Scroll line event (`wxScrollLineEvent`).

use crate::core::more_events::UiScrollAxis;

/// One scroll-line step (`wxScrollLineEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollLineEvent {
    pub axis: UiScrollAxis,
    pub lines: i32,
}

impl ScrollLineEvent {
    pub const fn new(axis: UiScrollAxis, lines: i32) -> Self {
        Self { axis, lines }
    }
}
