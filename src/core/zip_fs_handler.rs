//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Zip virtual file handler (`wxZipFSHandler`).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

static ZIP_FS: OnceLock<Mutex<HashMap<String, Vec<u8>>>> = OnceLock::new();

fn store() -> &'static Mutex<HashMap<String, Vec<u8>>> {
    ZIP_FS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register and resolve `zip:` URLs (`wxZipFSHandler`).
#[derive(Debug, Default, Clone, Copy)]
pub struct ZipFSHandler;

impl ZipFSHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn normalize_url(entry: &str) -> String {
        if entry.starts_with("zip:") {
            entry.to_string()
        } else {
            format!("zip:/{entry}")
        }
    }

    pub fn add_entry(&self, path: &str, data: Vec<u8>) {
        let url = Self::normalize_url(path);
        if let Ok(mut map) = store().lock() {
            map.insert(url, data);
        }
    }

    pub fn add_text(&self, path: &str, text: &str) {
        self.add_entry(path, text.as_bytes().to_vec());
    }

    pub fn get_entry(&self, path: &str) -> Option<Vec<u8>> {
        let url = Self::normalize_url(path);
        store().lock().ok()?.get(&url).cloned()
    }

    pub fn remove_entry(&self, path: &str) -> bool {
        let url = Self::normalize_url(path);
        store().lock().ok().is_some_and(|mut map| map.remove(&url).is_some())
    }

    pub fn list_entries(&self) -> Vec<crate::core::zip_entry::ZipEntry> {
        let Ok(map) = store().lock() else {
            return Vec::new();
        };
        map.iter()
            .map(|(path, data)| {
                let name = path.strip_prefix("zip:/").unwrap_or(path);
                crate::core::zip_entry::ZipEntry::new(name, data.len() as u64)
            })
            .collect()
    }
}
