//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! File control events (`wxFileCtrlEvent`).

/// File control selection changed (`wxFileCtrlEvent`).
#[derive(Debug, Clone)]
pub struct FileCtrlEvent {
    pub path: String,
}

impl FileCtrlEvent {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}
