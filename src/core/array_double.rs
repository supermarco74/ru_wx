//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Floating-point array (`wxArrayDouble`).

/// Growable `f64` list (`wxArrayDouble`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ArrayDouble {
    items: Vec<f64>,
}

impl ArrayDouble {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, value: f64) {
        self.items.push(value);
    }

    pub fn get(&self, index: usize) -> Option<f64> {
        self.items.get(index).copied()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn as_slice(&self) -> &[f64] {
        &self.items
    }
}
