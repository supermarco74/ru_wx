//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! In-memory virtual file handler (`wxMemoryFSHandler`).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

static MEMORY_FS: OnceLock<Mutex<HashMap<String, Vec<u8>>>> = OnceLock::new();

fn store() -> &'static Mutex<HashMap<String, Vec<u8>>> {
    MEMORY_FS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register and resolve `memory:` URLs (`wxMemoryFSHandler`).
#[derive(Debug, Default, Clone, Copy)]
pub struct MemoryFSHandler;

impl MemoryFSHandler {
    pub fn new() -> Self {
        Self
    }

    /// Normalise a path to `memory:/name` form.
    pub fn normalize_url(name: &str) -> String {
        if name.starts_with("memory:") {
            name.to_string()
        } else {
            format!("memory:/{name}")
        }
    }

    /// Store bytes under `memory:/name`.
    pub fn add_file(&self, name: &str, data: Vec<u8>) {
        let url = Self::normalize_url(name);
        if let Ok(mut map) = store().lock() {
            map.insert(url, data);
        }
    }

    /// Store UTF-8 text under `memory:/name`.
    pub fn add_text(&self, name: &str, text: &str) {
        self.add_file(name, text.as_bytes().to_vec());
    }

    /// Read bytes registered for `name`, if any.
    pub fn get_file(&self, name: &str) -> Option<Vec<u8>> {
        let url = Self::normalize_url(name);
        store().lock().ok()?.get(&url).cloned()
    }

    /// Remove a virtual file.
    pub fn remove_file(&self, name: &str) -> bool {
        let url = Self::normalize_url(name);
        store().lock().ok().is_some_and(|mut map| map.remove(&url).is_some())
    }

    /// Clear all in-memory entries.
    pub fn clear_all(&self) {
        if let Ok(mut map) = store().lock() {
            map.clear();
        }
    }
}

