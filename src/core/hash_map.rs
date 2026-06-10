//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! String hash map (`wxHashMap`).

use std::collections::HashMap as StdHashMap;

/// Key/value string map (`wxHashMap`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WxHashMap {
    items: StdHashMap<String, String>,
}

impl WxHashMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, key: &str, value: &str) -> Option<String> {
        self.items.insert(key.to_string(), value.to_string())
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.items.get(key).map(String::as_str)
    }

    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.items.remove(key)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.items.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}
