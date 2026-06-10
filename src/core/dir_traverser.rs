//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Directory traversal (`wxDirTraverser`).

use std::fs;
use std::path::{Path, PathBuf};

/// Callback trait for recursive directory walks.
pub trait DirTraverser {
    fn on_file(&mut self, path: &Path) -> bool;
    fn on_dir(&mut self, path: &Path) -> bool;
}

/// Collect every file path under `root`.
#[derive(Debug, Default, Clone)]
pub struct FileCollector {
    pub files: Vec<PathBuf>,
}

impl DirTraverser for FileCollector {
    fn on_file(&mut self, path: &Path) -> bool {
        self.files.push(path.to_path_buf());
        true
    }

    fn on_dir(&mut self, _path: &Path) -> bool {
        true
    }
}

/// Walk `root` depth-first, invoking `visitor` for each file and directory.
pub fn traverse_dir(root: &Path, visitor: &mut dyn DirTraverser) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if !visitor.on_dir(&path) {
                continue;
            }
            traverse_dir(&path, visitor);
        } else if !visitor.on_file(&path) {
            return;
        }
    }
}
