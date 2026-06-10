//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Zip archive entry (`wxZipEntry`).

/// Metadata for one zip archive member (`wxZipEntry`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZipEntry {
    pub path: String,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
}

impl ZipEntry {
    pub fn new(path: &str, data_len: u64) -> Self {
        Self {
            path: path.to_string(),
            compressed_size: data_len,
            uncompressed_size: data_len,
        }
    }

    pub fn name(&self) -> &str {
        &self.path
    }
}
