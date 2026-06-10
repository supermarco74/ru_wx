//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Temporary directory (`wxTempDir`).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Auto-removed temporary directory (`wxTempDir`).
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn new(prefix: &str) -> io::Result<Self> {
        let base = std::env::temp_dir();
        let path = base.join(format!("{prefix}_{}", std::process::id()));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn into_path(self) -> PathBuf {
        let path = self.path.clone();
        std::mem::forget(self);
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
