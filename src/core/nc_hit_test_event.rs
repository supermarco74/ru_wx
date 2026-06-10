//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Non-client hit-test event (`wxNcHitTestEvent`).

use crate::core::geometry::Point;

/// Hit-test over the non-client area (`wxNcHitTestEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NcHitTestEvent {
    pub position: Point,
    pub hit_code: i32,
}

impl NcHitTestEvent {
    pub const fn new(position: Point, hit_code: i32) -> Self {
        Self {
            position,
            hit_code,
        }
    }
}
