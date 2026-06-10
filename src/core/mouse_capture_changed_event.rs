//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Mouse capture changed event (`wxMouseCaptureChangedEvent`).

/// Mouse capture moved to another window (`wxMouseCaptureChangedEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseCaptureChangedEvent {
    pub captured: bool,
    pub window_id: u16,
}

impl MouseCaptureChangedEvent {
    pub const fn new(captured: bool, window_id: u16) -> Self {
        Self {
            captured,
            window_id,
        }
    }
}
