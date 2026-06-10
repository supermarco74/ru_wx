//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Integer array (`wxArrayInt`).

/// Growable `i32` list (`wxArrayInt`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArrayInt {
    items: Vec<i32>,
}

impl ArrayInt {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, value: i32) {
        self.items.push(value);
    }

    pub fn insert(&mut self, index: usize, value: i32) {
        self.items.insert(index, value);
    }

    pub fn get(&self, index: usize) -> Option<i32> {
        self.items.get(index).copied()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn as_slice(&self) -> &[i32] {
        &self.items
    }
}
