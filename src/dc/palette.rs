//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Colour palette (`wxPalette`).

use crate::core::geometry::Colour;

/// Indexed colour palette (`wxPalette`).
#[derive(Debug, Clone)]
pub struct Palette {
    entries: Vec<Colour>,
}

impl Palette {
    pub fn new(entries: &[Colour]) -> Self {
        Self {
            entries: entries.to_vec(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<Colour> {
        self.entries.get(index).copied()
    }

    pub fn entries(&self) -> &[Colour] {
        &self.entries
    }
}
