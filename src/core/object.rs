//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Base object identity (`wxObject`).

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_OBJECT_ID: AtomicU64 = AtomicU64::new(1);

/// Root identity for wx-like object hierarchies (`wxObject`).
#[derive(Debug)]
pub struct WxObject {
    id: u64,
}

impl WxObject {
    pub fn new() -> Self {
        Self {
            id: NEXT_OBJECT_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }
}

impl Default for WxObject {
    fn default() -> Self {
        Self::new()
    }
}
