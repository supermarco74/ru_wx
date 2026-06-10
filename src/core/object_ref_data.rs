//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Shared object payload (`wxObjectRefData`).

use crate::core::ref_counter::RefCounter;
use crate::core::variant::Variant;

/// Reference-counted object data (`wxObjectRefData`).
#[derive(Debug)]
pub struct ObjectRefData {
    refs: RefCounter,
    pub payload: Variant,
}

impl ObjectRefData {
    pub fn new(payload: Variant) -> Self {
        Self {
            refs: RefCounter::with_count(1),
            payload,
        }
    }

    pub fn add_ref(&self) -> usize {
        self.refs.inc()
    }

    pub fn release(&self) -> usize {
        self.refs.dec()
    }

    pub fn ref_count(&self) -> usize {
        self.refs.get()
    }
}

impl Clone for ObjectRefData {
    fn clone(&self) -> Self {
        self.add_ref();
        Self {
            refs: RefCounter::with_count(self.refs.get()),
            payload: self.payload.clone(),
        }
    }
}
