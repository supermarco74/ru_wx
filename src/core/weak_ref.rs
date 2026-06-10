//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Weak reference helper (`wxWeakRef`).

use std::rc::{Rc, Weak};

/// Non-owning handle to a shared object (`wxWeakRef`).
#[derive(Debug, Clone)]
pub struct WeakRef<T> {
    inner: Weak<T>,
}

impl<T> WeakRef<T> {
    pub fn new(strong: &Rc<T>) -> Self {
        Self {
            inner: Rc::downgrade(strong),
        }
    }

    pub fn upgrade(&self) -> Option<Rc<T>> {
        self.inner.upgrade()
    }

    pub fn is_expired(&self) -> bool {
        self.inner.strong_count() == 0
    }
}
