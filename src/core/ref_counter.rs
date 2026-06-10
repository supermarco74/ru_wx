//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Reference counter (`wxRefCounter`).

use std::sync::atomic::{AtomicUsize, Ordering};

/// Thread-safe reference count (`wxRefCounter`).
#[derive(Debug, Default)]
pub struct RefCounter {
    count: AtomicUsize,
}

impl RefCounter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_count(initial: usize) -> Self {
        Self {
            count: AtomicUsize::new(initial),
        }
    }

    pub fn inc(&self) -> usize {
        self.count.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn dec(&self) -> usize {
        self.count.fetch_sub(1, Ordering::Relaxed).saturating_sub(1)
    }

    pub fn get(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    pub fn is_unique(&self) -> bool {
        self.get() <= 1
    }
}
