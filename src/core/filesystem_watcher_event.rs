//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! File-system watcher event (`wxFileSystemWatcherEvent`).

/// Directory or file change notification (`wxFileSystemWatcherEvent`).
#[derive(Debug, Clone)]
pub struct FileSystemWatcherEvent {
    pub path: String,
    pub change_type: FileSystemChangeType,
}

/// Kind of filesystem change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSystemChangeType {
    Create,
    Delete,
    Modify,
    Rename,
}

impl FileSystemWatcherEvent {
    pub fn new(path: impl Into<String>, change_type: FileSystemChangeType) -> Self {
        Self {
            path: path.into(),
            change_type,
        }
    }
}
