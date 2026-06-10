//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Non-client paint event (`wxNcPaintEvent`).

use crate::core::geometry::Rect;

/// Non-client area needs painting (`wxNcPaintEvent`).
#[derive(Debug, Clone, Copy)]
pub struct NcPaintEvent {
    pub rect: Rect,
}

impl NcPaintEvent {
    pub const fn new(rect: Rect) -> Self {
        Self { rect }
    }
}
