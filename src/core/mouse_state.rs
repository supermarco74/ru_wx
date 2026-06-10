//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Mouse button state (`wxMouseState`).

use crate::core::geometry::Point;
use crate::core::input_events::{KeyModifiers, MouseEvent};

/// Snapshot of mouse buttons and modifiers (`wxMouseState`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MouseState {
    pub position: Point,
    pub left_down: bool,
    pub middle_down: bool,
    pub right_down: bool,
    pub modifiers: KeyModifiers,
}

impl MouseState {
    pub const fn new(position: Point) -> Self {
        Self {
            position,
            left_down: false,
            middle_down: false,
            right_down: false,
            modifiers: KeyModifiers::empty(),
        }
    }

    pub fn from_mouse_event(ev: &MouseEvent) -> Self {
        Self {
            position: ev.position,
            left_down: ev.left_is_down(),
            middle_down: ev.button_state & 0x0010 != 0,
            right_down: ev.button_state & 0x0002 != 0,
            modifiers: KeyModifiers::empty(),
        }
    }
}
