//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Search-path list (`wxPathList`).

use std::env;
use std::path::{Path, PathBuf};

/// Ordered list of directories searched for relative paths (`wxPathList`).
#[derive(Debug, Clone, Default)]
pub struct PathList {
    paths: Vec<PathBuf>,
}

impl PathList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, path: impl AsRef<Path>) {
        self.paths.push(path.as_ref().to_path_buf());
    }

    pub fn add_env_path(&mut self, var: &str) {
        if let Ok(value) = env::var(var) {
            #[cfg(target_os = "windows")]
            let sep = ';';
            #[cfg(not(target_os = "windows"))]
            let sep = ':';
            for part in value.split(sep) {
                if !part.is_empty() {
                    self.add(part);
                }
            }
        }
    }

    pub fn find_valid_path(&self, filename: &str) -> Option<PathBuf> {
        for base in &self.paths {
            let candidate = base.join(filename);
            if candidate.exists() {
                return Some(candidate);
            }
        }
        None
    }

    pub fn as_slice(&self) -> &[PathBuf] {
        &self.paths
    }
}
