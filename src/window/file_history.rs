//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Recent-files list (`wxFileHistory`).

use std::collections::VecDeque;

/// MRU file list for menus (`wxFileHistory`).
#[derive(Debug, Clone)]
pub struct FileHistory {
    max_files: usize,
    files: VecDeque<String>,
}

impl FileHistory {
    pub fn new(max_files: usize) -> Self {
        Self {
            max_files: max_files.max(1),
            files: VecDeque::new(),
        }
    }

    pub fn add_file(&mut self, path: &str) {
        self.files.retain(|p| p != path);
        self.files.push_front(path.to_string());
        while self.files.len() > self.max_files {
            self.files.pop_back();
        }
    }

    pub fn files(&self) -> Vec<&str> {
        self.files.iter().map(|s| s.as_str()).collect()
    }

    pub fn file_at(&self, index: usize) -> Option<&str> {
        self.files.get(index).map(|s| s.as_str())
    }

    pub fn count(&self) -> usize {
        self.files.len()
    }

    pub fn clear(&mut self) {
        self.files.clear();
    }
}
