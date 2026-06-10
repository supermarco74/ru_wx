//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! String list (`wxStringList`).

/// Linked-style string list (`wxStringList`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StringList {
    items: Vec<String>,
}

impl StringList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_slice(slice: &[&str]) -> Self {
        Self {
            items: slice.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    pub fn append(&mut self, value: &str) {
        self.items.push(value.to_string());
    }

    pub fn prepend(&mut self, value: &str) {
        self.items.insert(0, value.to_string());
    }

    pub fn first(&self) -> Option<&str> {
        self.items.first().map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn into_vec(self) -> Vec<String> {
        self.items
    }
}
