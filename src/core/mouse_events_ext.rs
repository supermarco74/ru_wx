//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Extended mouse events (`wxMouseEnterWindow`, …).

use crate::core::geometry::Point;

/// Mouse entered window client area (`wxMouseEnterWindow`).
#[derive(Debug, Clone, Copy)]
pub struct MouseEnterEvent {
    pub position: Point,
}

impl MouseEnterEvent {
    pub const fn new(position: Point) -> Self {
        Self { position }
    }
}

/// Mouse left window client area (`wxMouseLeaveWindow`).
#[derive(Debug, Clone, Copy, Default)]
pub struct MouseLeaveEvent;

impl MouseLeaveEvent {
    pub const fn new() -> Self {
        Self
    }
}

/// Capture lost (`wxMouseCaptureLostEvent`).
#[derive(Debug, Clone, Copy, Default)]
pub struct MouseCaptureLostEvent;

impl MouseCaptureLostEvent {
    pub const fn new() -> Self {
        Self
    }
}
