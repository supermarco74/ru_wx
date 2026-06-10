//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! String hash set (`wxHashSet`).

use std::collections::HashSet as StdHashSet;

/// Unique string collection (`wxHashSet`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WxHashSet {
    items: StdHashSet<String>,
}

impl WxHashSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, value: &str) -> bool {
        self.items.insert(value.to_string())
    }

    pub fn contains(&self, value: &str) -> bool {
        self.items.contains(value)
    }

    pub fn remove(&mut self, value: &str) -> bool {
        self.items.remove(value)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.items.iter()
    }
}
