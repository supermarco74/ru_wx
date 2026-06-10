//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Display configuration change event (`wxDisplayChangedEvent`).

/// Monitor layout or DPI changed (`wxDisplayChangedEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayChangedEvent {
    pub display_index: u32,
}

impl DisplayChangedEvent {
    pub const fn new(display_index: u32) -> Self {
        Self { display_index }
    }
}
