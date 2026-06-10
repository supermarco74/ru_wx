//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Context menu event (`wxContextMenuEvent`).

use crate::core::geometry::Point;

/// Right-click / keyboard menu (`wxContextMenuEvent`).
#[derive(Debug, Clone, Copy)]
pub struct ContextMenuEvent {
    pub position: Point,
    pub is_keyboard: bool,
}

impl ContextMenuEvent {
    pub const fn mouse(position: Point) -> Self {
        Self {
            position,
            is_keyboard: false,
        }
    }

    pub const fn keyboard(position: Point) -> Self {
        Self {
            position,
            is_keyboard: true,
        }
    }
}
