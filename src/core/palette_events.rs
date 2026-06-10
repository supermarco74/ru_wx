//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Palette change events (`wxPaletteChangedEvent`, …).

/// System palette changed (`wxPaletteChangedEvent`).
#[derive(Debug, Clone, Copy, Default)]
pub struct PaletteChangedEvent;

impl PaletteChangedEvent {
    pub const fn new() -> Self {
        Self
    }
}

/// Window needs palette realization (`wxQueryNewPaletteEvent`).
#[derive(Debug, Clone, Copy, Default)]
pub struct QueryNewPaletteEvent;

impl QueryNewPaletteEvent {
    pub const fn new() -> Self {
        Self
    }
}
