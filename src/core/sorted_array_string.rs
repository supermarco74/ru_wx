//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Sorted string array (`wxSortedArrayString`).

use crate::core::array_string::ArrayString;

/// String list kept in sorted order (`wxSortedArrayString`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SortedArrayString {
    inner: ArrayString,
    case_sensitive: bool,
}

impl SortedArrayString {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_case_sensitive(mut self, case_sensitive: bool) -> Self {
        self.case_sensitive = case_sensitive;
        self
    }

    pub fn add(&mut self, value: &str) {
        let mut items: Vec<String> = self.inner.as_slice().to_vec();
        items.push(value.to_string());
        self.sort_items(&mut items);
        self.inner = ArrayString::from_vec(items);
    }

    pub fn index(&self, value: &str) -> Option<usize> {
        let items = self.inner.as_slice();
        if self.case_sensitive {
            items.iter().position(|s| s == value)
        } else {
            let lower = value.to_ascii_lowercase();
            items
                .iter()
                .position(|s| s.to_ascii_lowercase() == lower)
        }
    }

    pub fn get(&self, index: usize) -> Option<&str> {
        self.inner.get(index)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn sort_items(&self, items: &mut [String]) {
        if self.case_sensitive {
            items.sort();
        } else {
            items.sort_by_key(|s| s.to_ascii_lowercase());
        }
    }
}
