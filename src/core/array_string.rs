//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! String array (`wxArrayString`).

use std::fmt;

/// Growable string list (`wxArrayString`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArrayString {
    items: Vec<String>,
}

impl ArrayString {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
        }
    }

    pub fn from_slice(slice: &[&str]) -> Self {
        Self {
            items: slice.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    pub fn from_vec(items: Vec<String>) -> Self {
        Self { items }
    }

    pub fn add(&mut self, value: &str) {
        self.items.push(value.to_string());
    }

    pub fn insert(&mut self, index: usize, value: &str) {
        self.items.insert(index, value.to_string());
    }

    pub fn remove(&mut self, index: usize) -> Option<String> {
        if index < self.items.len() {
            Some(self.items.remove(index))
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&str> {
        self.items.get(index).map(String::as_str)
    }

    pub fn index(&self, value: &str) -> Option<usize> {
        self.items.iter().position(|s| s == value)
    }

    pub fn join(&self, sep: &str) -> String {
        self.items.join(sep)
    }

    pub fn as_slice(&self) -> &[String] {
        &self.items
    }
}

impl fmt::Display for ArrayString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.join("; "))
    }
}
