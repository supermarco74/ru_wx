//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Non-client size calculation event (`wxNcCalcSizeEvent`).

use crate::core::geometry::Rect;

/// Non-client area size calculation (`wxNcCalcSizeEvent`).
#[derive(Debug, Clone, Copy)]
pub struct NcCalcSizeEvent {
    pub rect: Rect,
    pub valid: bool,
}

impl NcCalcSizeEvent {
    pub const fn new(rect: Rect, valid: bool) -> Self {
        Self { rect, valid }
    }
}
