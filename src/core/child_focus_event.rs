//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Child focus event (`wxChildFocusEvent`).

/// Focus moved to a child window (`wxChildFocusEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChildFocusEvent {
    pub child_id: u16,
}

impl ChildFocusEvent {
    pub const fn new(child_id: u16) -> Self {
        Self { child_id }
    }
}
