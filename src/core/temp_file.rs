//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Temporary file (`wxTempFile`).

use std::fs::{self, File};
use std::io;
use std::path::PathBuf;

/// Auto-deleted temp file (`wxTempFile`).
pub struct TempFile {
    path: PathBuf,
    file: Option<File>,
}

impl TempFile {
    pub fn new(prefix: &str) -> io::Result<Self> {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("{prefix}_{}", std::process::id()));
        let file = File::create(&path)?;
        Ok(Self {
            path,
            file: Some(file),
        })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn take_file(&mut self) -> Option<File> {
        self.file.take()
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
