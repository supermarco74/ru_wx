//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Long integer array (`wxArrayLong`).

/// Growable `i64` list (`wxArrayLong`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArrayLong {
    items: Vec<i64>,
}

impl ArrayLong {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, value: i64) {
        self.items.push(value);
    }

    pub fn get(&self, index: usize) -> Option<i64> {
        self.items.get(index).copied()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn as_slice(&self) -> &[i64] {
        &self.items
    }
}
