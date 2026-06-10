//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Notebook / book control events (`wxBookCtrlEvent`).

/// Page selection changed in a notebook-like control (`wxBookCtrlEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BookCtrlEvent {
    pub old_page: usize,
    pub new_page: usize,
}

impl BookCtrlEvent {
    pub const fn new(old_page: usize, new_page: usize) -> Self {
        Self { old_page, new_page }
    }
}
