//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! INI file config (`wxFileConfig`).

use std::path::{Path, PathBuf};

use crate::core::config::Config;

/// File-backed INI config (`wxFileConfig`).
#[derive(Debug)]
pub struct FileConfig {
    path: PathBuf,
    inner: Config,
}

impl FileConfig {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        Self {
            inner: Config::new(&path),
            path,
        }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn read_str(&self, key: &str, default: &str) -> String {
        self.inner.read(key, default)
    }

    pub fn write_str(&mut self, key: &str, value: &str) {
        self.inner.write(key, value);
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
