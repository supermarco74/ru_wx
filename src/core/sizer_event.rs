//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Sizer layout events (`wxSizerEvent`).

use crate::core::geometry::Size;

/// Layout recalculated (`wxSizerEvent`).
#[derive(Debug, Clone, Copy)]
pub struct SizerEvent {
    pub new_size: Size,
}

impl SizerEvent {
    pub const fn new(new_size: Size) -> Self {
        Self { new_size }
    }
}
