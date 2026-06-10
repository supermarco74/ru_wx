//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Header control events (`wxHeaderButtonClickEvent`, …).

/// Header column button clicked (`wxHeaderButtonClickEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeaderButtonClickEvent {
    pub column: usize,
}

impl HeaderButtonClickEvent {
    pub const fn new(column: usize) -> Self {
        Self { column }
    }
}

/// Header column resized or reordered (`wxHeaderColumnEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeaderColumnEvent {
    pub column: usize,
    pub width: u32,
}

impl HeaderColumnEvent {
    pub const fn new(column: usize, width: u32) -> Self {
        Self { column, width }
    }
}

