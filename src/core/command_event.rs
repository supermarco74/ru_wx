//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Command events (`wxCommandEvent`).

/// Button click, menu selection, accelerator (`wxCommandEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandEvent {
    pub id: u16,
    pub checked: bool,
}

impl CommandEvent {
    pub const fn new(id: u16) -> Self {
        Self { id, checked: false }
    }

    pub const fn with_checked(id: u16, checked: bool) -> Self {
        Self { id, checked }
    }
}
