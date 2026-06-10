//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Property grid change event (`wxPropertyGridEvent`).

/// Property value changed in a grid (`wxPropertyGridEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropertyGridEvent {
    pub property_index: usize,
    pub changed: bool,
}

impl PropertyGridEvent {
    pub const fn new(property_index: usize, changed: bool) -> Self {
        Self {
            property_index,
            changed,
        }
    }
}

