//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Additional system/window events (round 20).

use crate::core::geometry::Point;

/// System colour palette changed (`wxSysColourChangedEvent`).
#[derive(Debug, Clone, Copy, Default)]
pub struct SysColourChangedEvent;

impl SysColourChangedEvent {
    pub const fn new() -> Self {
        Self
    }
}

/// Joystick input (`wxJoystickEvent`).
#[derive(Debug, Clone, Copy)]
pub struct JoystickEvent {
    pub button: u32,
    pub position: Point,
    pub z_position: i32,
    pub is_move: bool,
}

impl JoystickEvent {
    pub const fn button_press(button: u32, position: Point) -> Self {
        Self {
            button,
            position,
            z_position: 0,
            is_move: false,
        }
    }

    pub const fn move_event(position: Point, z: i32) -> Self {
        Self {
            button: 0,
            position,
            z_position: z,
            is_move: true,
        }
    }
}

/// Cursor set request (`wxSetCursorEvent`).
#[derive(Debug, Clone, Copy)]
pub struct SetCursorEvent {
    pub hit_test: i32,
    pub position: Point,
}

impl SetCursorEvent {
    pub const fn new(hit_test: i32, position: Point) -> Self {
        Self { hit_test, position }
    }
}

/// DPI change (`wxDpiChangedEvent`).
#[derive(Debug, Clone, Copy)]
pub struct DpiChangedEvent {
    pub old_dpi: u32,
    pub new_dpi: u32,
    pub suggested_rect: crate::core::geometry::Rect,
}

impl DpiChangedEvent {
    pub const fn new(old_dpi: u32, new_dpi: u32, suggested_rect: crate::core::geometry::Rect) -> Self {
        Self {
            old_dpi,
            new_dpi,
            suggested_rect,
        }
    }
}

/// Full-screen toggle (`wxFullScreenEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FullScreenEvent {
    pub full_screen: bool,
}

impl FullScreenEvent {
    pub const fn entered() -> Self {
        Self { full_screen: true }
    }

    pub const fn exited() -> Self {
        Self { full_screen: false }
    }
}

/// Generic scroll (`wxScrollEvent`) — distinct from [`crate::ScrollWinEvent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiScrollAxis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy)]
pub struct UiScrollEvent {
    pub axis: UiScrollAxis,
    pub position: i32,
    pub event_type: u16,
}

impl UiScrollEvent {
    pub const fn new(axis: UiScrollAxis, position: i32, event_type: u16) -> Self {
        Self {
            axis,
            position,
            event_type,
        }
    }
}
