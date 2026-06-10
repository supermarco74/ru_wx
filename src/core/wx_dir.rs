//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Directory helper (`wxDir`).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Directory operations (`wxDir`).
#[derive(Debug, Clone)]
pub struct WxDir {
    path: PathBuf,
}

impl WxDir {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn exists(&self) -> bool {
        self.path.is_dir()
    }

    pub fn make(&self) -> io::Result<()> {
        fs::create_dir_all(&self.path)
    }

    pub fn list_files(&self) -> io::Result<Vec<PathBuf>> {
        let mut out = Vec::new();
        for entry in fs::read_dir(&self.path)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                out.push(entry.path());
            }
        }
        Ok(out)
    }

    pub fn list_subdirs(&self) -> io::Result<Vec<PathBuf>> {
        let mut out = Vec::new();
        for entry in fs::read_dir(&self.path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                out.push(entry.path());
            }
        }
        Ok(out)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
