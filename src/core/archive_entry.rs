//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Archive entry metadata (`wxArchiveEntry`).

/// Metadata for one archive member (`wxArchiveEntry`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveEntry {
    pub path: String,
    pub size: u64,
}

impl ArchiveEntry {
    pub fn new(path: &str, size: u64) -> Self {
        Self {
            path: path.to_string(),
            size,
        }
    }

    pub fn name(&self) -> &str {
        &self.path
    }
}
