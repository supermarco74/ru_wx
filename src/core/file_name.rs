//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Path manipulation (`wxFileName`).

use std::path::{Component, Path, PathBuf};

/// Cross-platform path helper (`wxFileName`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileName {
    path: PathBuf,
}

impl FileName {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn file_name(&self) -> Option<String> {
        self.path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
    }

    pub fn extension(&self) -> Option<String> {
        self.path
            .extension()
            .map(|s| s.to_string_lossy().into_owned())
    }

    pub fn dir(&self) -> Option<PathBuf> {
        self.path.parent().map(Path::to_path_buf)
    }

    pub fn is_absolute(&self) -> bool {
        self.path.is_absolute()
    }

    pub fn append_dir(&mut self, segment: &str) {
        self.path.push(segment);
    }

    pub fn set_full_name(&mut self, name: &str) {
        if let Some(parent) = self.path.parent() {
            self.path = parent.join(name);
        } else {
            self.path = PathBuf::from(name);
        }
    }

    pub fn normalize(&mut self) {
        let mut out = PathBuf::new();
        for comp in self.path.components() {
            match comp {
                Component::ParentDir => {
                    out.pop();
                }
                Component::CurDir => {}
                other => out.push(other.as_os_str()),
            }
        }
        self.path = out;
    }

    pub fn as_path(&self) -> &Path {
        &self.path
    }

    pub fn into_path_buf(self) -> PathBuf {
        self.path
    }
}
