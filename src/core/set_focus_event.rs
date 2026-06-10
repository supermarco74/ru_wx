//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Focus-gained event (`wxSetFocusEvent`).

/// Focus entered a window (`wxSetFocusEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetFocusEvent {
    pub window_id: u16,
}

impl SetFocusEvent {
    pub const fn new(window_id: u16) -> Self {
        Self { window_id }
    }
}
