//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Window lifetime events (`wxWindowCreateEvent`, …).

/// Child or top-level window created (`wxWindowCreateEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowCreateEvent {
    pub id: u16,
}

impl WindowCreateEvent {
    pub const fn new(id: u16) -> Self {
        Self { id }
    }
}

/// Window being destroyed (`wxWindowDestroyEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowDestroyEvent {
    pub id: u16,
}

impl WindowDestroyEvent {
    pub const fn new(id: u16) -> Self {
        Self { id }
    }
}
