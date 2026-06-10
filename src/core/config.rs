//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Configuration and paths (`wxConfig`, `wxStandardPaths`, …).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Key/value settings store (`wxConfig`).
#[derive(Debug, Default)]
pub struct Config {
    path: PathBuf,
    values: HashMap<String, String>,
    dirty: bool,
}

impl Config {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        let mut cfg = Self {
            path,
            values: HashMap::new(),
            dirty: false,
        };
        cfg.read_disk();
        cfg
    }

    pub fn read(&self, key: &str, default: &str) -> String {
        self.values.get(key).cloned().unwrap_or_else(|| default.to_string())
    }

    pub fn write(&mut self, key: &str, value: &str) {
        self.values.insert(key.to_string(), value.to_string());
        self.dirty = true;
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        if !self.dirty {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let body: String = self
            .values
            .iter()
            .map(|(k, v)| format!("{k}={v}\n"))
            .collect();
        fs::write(&self.path, body)?;
        self.dirty = false;
        Ok(())
    }

    fn read_disk(&mut self) {
        if let Ok(text) = fs::read_to_string(&self.path) {
            for line in text.lines() {
                if let Some((k, v)) = line.split_once('=') {
                    self.values.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
        }
    }
}

/// Well-known application directories (`wxStandardPaths`).
#[derive(Debug, Clone)]
pub struct StandardPaths {
    app_name: String,
}

impl StandardPaths {
    pub fn new(app_name: &str) -> Self {
        Self {
            app_name: app_name.to_string(),
        }
    }

    pub fn config_dir(&self) -> PathBuf {
        dirs_config().join(&self.app_name)
    }

    pub fn data_dir(&self) -> PathBuf {
        dirs_data().join(&self.app_name)
    }

    pub fn user_config_file(&self, leaf: &str) -> PathBuf {
        self.config_dir().join(leaf)
    }
}

fn dirs_config() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn dirs_data() -> PathBuf {
    dirs_config()
}

/// Locale placeholder (`wxLocale`).
#[derive(Debug, Clone, Copy, Default)]
pub struct Locale {
    language: u32,
}

impl Locale {
    pub const fn new(language: u32) -> Self {
        Self { language }
    }

    pub const fn language(&self) -> u32 {
        self.language
    }
}

/// Replaceable watcher event callback slot.
type WatcherEventHandler = std::cell::RefCell<
    Option<Box<dyn FnMut(&crate::core::filesystem_watcher_event::FileSystemWatcherEvent)>>,
>;

/// Directory change notifications (`wxFileSystemWatcher`) — stub.
pub struct FileSystemWatcher {
    paths: Vec<PathBuf>,
    on_event: WatcherEventHandler,
}

impl Default for FileSystemWatcher {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            on_event: std::cell::RefCell::new(None),
        }
    }
}

impl std::fmt::Debug for FileSystemWatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileSystemWatcher")
            .field("paths", &self.paths)
            .finish_non_exhaustive()
    }
}

impl FileSystemWatcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, path: impl AsRef<Path>) {
        self.paths.push(path.as_ref().to_path_buf());
    }

    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    /// Register a callback for filesystem change notifications.
    pub fn on_event<F: FnMut(&crate::core::filesystem_watcher_event::FileSystemWatcherEvent) + 'static>(
        &self,
        f: F,
    ) {
        *self.on_event.borrow_mut() = Some(Box::new(f));
    }

    /// Simulate a change event (stub until native watcher backend).
    pub fn notify_change(
        &self,
        path: impl AsRef<Path>,
        change_type: crate::core::filesystem_watcher_event::FileSystemChangeType,
    ) {
        if let Some(ref mut cb) = *self.on_event.borrow_mut() {
            cb(&crate::core::filesystem_watcher_event::FileSystemWatcherEvent::new(
                path.as_ref().display().to_string(),
                change_type,
            ));
        }
    }
}

/// Single-instance guard (`wxSingleInstanceChecker`).
#[derive(Debug)]
pub struct SingleInstanceChecker {
    name: String,
}

impl SingleInstanceChecker {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    pub fn is_another_running(&self) -> bool {
        let lock = StandardPaths::new(&self.name).config_dir().join("instance.lock");
        if lock.exists() {
            return true;
        }
        if let Some(parent) = lock.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&lock, std::process::id().to_string());
        false
    }
}
