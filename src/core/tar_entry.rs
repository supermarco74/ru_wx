//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Tar archive entry (`wxTarEntry`).

/// Metadata for one tar member (`wxTarEntry`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TarEntry {
    pub path: String,
    pub size: u64,
    pub is_directory: bool,
}

impl TarEntry {
    pub fn new(path: &str, size: u64) -> Self {
        Self {
            path: path.to_string(),
            size,
            is_directory: false,
        }
    }

    pub fn directory(path: &str) -> Self {
        Self {
            path: path.to_string(),
            size: 0,
            is_directory: true,
        }
    }
}
