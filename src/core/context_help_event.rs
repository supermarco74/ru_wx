//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Context-help events (`wxContextHelpEvent`).

use crate::core::geometry::Point;

/// Fired when the user requests context-sensitive help (F1 on a control).
#[derive(Debug, Clone, Copy)]
pub struct ContextHelpEvent {
    pub control_id: u16,
    pub position: Point,
}

impl ContextHelpEvent {
    pub const fn new(control_id: u16, position: Point) -> Self {
        Self {
            control_id,
            position,
        }
    }
}
