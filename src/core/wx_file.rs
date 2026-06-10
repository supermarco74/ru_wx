//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! High-level file helper (`wxFile`).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// File existence and copy helpers (`wxFile`).
#[derive(Debug, Clone)]
pub struct WxFile {
    path: PathBuf,
}

impl WxFile {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    pub fn is_file(&self) -> bool {
        self.path.is_file()
    }

    pub fn length(&self) -> io::Result<u64> {
        Ok(fs::metadata(&self.path)?.len())
    }

    pub fn modification_time(&self) -> io::Result<SystemTime> {
        fs::metadata(&self.path)?.modified()
    }

    pub fn read_all(&self) -> io::Result<Vec<u8>> {
        fs::read(&self.path)
    }

    pub fn read_to_string(&self) -> io::Result<String> {
        fs::read_to_string(&self.path)
    }

    pub fn write_all(&self, data: &[u8]) -> io::Result<()> {
        fs::write(&self.path, data)
    }

    pub fn copy_to(&self, dest: impl AsRef<Path>) -> io::Result<u64> {
        fs::copy(&self.path, dest)
    }

    pub fn remove(&self) -> io::Result<()> {
        fs::remove_file(&self.path)
    }

    pub fn rename(&self, dest: impl AsRef<Path>) -> io::Result<()> {
        fs::rename(&self.path, dest)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
