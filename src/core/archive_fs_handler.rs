//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Archive virtual file handler (`wxArchiveFSHandler`).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

static ARCHIVE_FS: OnceLock<Mutex<HashMap<String, Vec<u8>>>> = OnceLock::new();

fn store() -> &'static Mutex<HashMap<String, Vec<u8>>> {
    ARCHIVE_FS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register and resolve `archive:` URLs (`wxArchiveFSHandler`).
#[derive(Debug, Default, Clone, Copy)]
pub struct ArchiveFSHandler;

impl ArchiveFSHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn normalize_url(name: &str) -> String {
        if name.starts_with("archive:") {
            name.to_string()
        } else {
            format!("archive:/{name}")
        }
    }

    pub fn add_file(&self, name: &str, data: Vec<u8>) {
        let url = Self::normalize_url(name);
        if let Ok(mut map) = store().lock() {
            map.insert(url, data);
        }
    }

    pub fn add_text(&self, name: &str, text: &str) {
        self.add_file(name, text.as_bytes().to_vec());
    }

    pub fn get_file(&self, name: &str) -> Option<Vec<u8>> {
        let url = Self::normalize_url(name);
        store().lock().ok()?.get(&url).cloned()
    }

    pub fn remove_file(&self, name: &str) -> bool {
        let url = Self::normalize_url(name);
        store().lock().ok().is_some_and(|mut map| map.remove(&url).is_some())
    }

    pub fn list_entries(&self) -> Vec<crate::core::archive_entry::ArchiveEntry> {
        let Ok(map) = store().lock() else {
            return Vec::new();
        };
        map.iter()
            .map(|(path, data)| {
                let name = path.strip_prefix("archive:/").unwrap_or(path);
                crate::core::archive_entry::ArchiveEntry::new(name, data.len() as u64)
            })
            .collect()
    }
}
