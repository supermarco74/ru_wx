//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Mouse wheel event (`wxMouseWheelEvent`).

use crate::core::geometry::Point;
use crate::core::input_events::KeyModifiers;

/// Mouse wheel scrolled (`wxMouseWheelEvent`).
#[derive(Debug, Clone, Copy)]
pub struct MouseWheelEvent {
    pub position: Point,
    pub delta: i16,
    pub modifiers: KeyModifiers,
    pub horizontal: bool,
}

impl MouseWheelEvent {
    pub const fn new(position: Point, delta: i16) -> Self {
        Self {
            position,
            delta,
            modifiers: KeyModifiers::empty(),
            horizontal: false,
        }
    }

    pub const fn with_modifiers(mut self, modifiers: KeyModifiers) -> Self {
        self.modifiers = modifiers;
        self
    }
}
