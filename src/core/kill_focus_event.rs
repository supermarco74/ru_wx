//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Focus-lost event (`wxKillFocusEvent`).

/// Focus left a window (`wxKillFocusEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KillFocusEvent {
    pub window_id: u16,
}

impl KillFocusEvent {
    pub const fn new(window_id: u16) -> Self {
        Self { window_id }
    }
}
