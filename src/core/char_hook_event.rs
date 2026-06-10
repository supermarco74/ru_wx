//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Character hook (`wxCharHookEvent`).

/// Intercept keys before child controls (`wxCharHookEvent`).
#[derive(Debug)]
pub struct CharHookEvent {
    pub unicode: u32,
    vetoed: std::cell::Cell<bool>,
}

impl CharHookEvent {
    pub const fn new(unicode: u32) -> Self {
        Self {
            unicode,
            vetoed: std::cell::Cell::new(false),
        }
    }

    pub fn veto(&self) {
        self.vetoed.set(true);
    }

    pub fn is_vetoed(&self) -> bool {
        self.vetoed.get()
    }
}
