//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Notebook page events (`wxNotebookEvent`).

/// Tab/page changed (`wxNotebookEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotebookEvent {
    pub page: usize,
}

impl NotebookEvent {
    pub const fn new(page: usize) -> Self {
        Self { page }
    }
}
