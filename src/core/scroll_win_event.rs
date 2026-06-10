//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Scrolled-window events (`wxScrollWinEvent`).

/// Scroll position change in a scrolled window (`wxScrollWinEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollWinAxis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy)]
pub struct ScrollWinEvent {
    pub axis: ScrollWinAxis,
    pub position: i32,
}

impl ScrollWinEvent {
    pub const fn new(axis: ScrollWinAxis, position: i32) -> Self {
        Self { axis, position }
    }
}
