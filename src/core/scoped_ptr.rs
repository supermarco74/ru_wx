//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Scoped unique pointer (`wxScopedPtr`).

/// Owns a value and deletes it on drop (`wxScopedPtr`).
pub struct ScopedPtr<T> {
    inner: Option<T>,
}

impl<T> ScopedPtr<T> {
    pub fn new(value: T) -> Self {
        Self { inner: Some(value) }
    }

    pub fn get(&self) -> Option<&T> {
        self.inner.as_ref()
    }

    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.inner.as_mut()
    }

    pub fn reset(&mut self, value: T) -> Option<T> {
        self.inner.replace(value)
    }

    pub fn take(&mut self) -> Option<T> {
        self.inner.take()
    }

    pub fn is_null(&self) -> bool {
        self.inner.is_none()
    }
}

impl<T> Drop for ScopedPtr<T> {
    fn drop(&mut self) {
        self.inner = None;
    }
}
