//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Internet virtual file handler (`wxInternetFSHandler`).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

static INTERNET_FS: OnceLock<Mutex<HashMap<String, Vec<u8>>>> = OnceLock::new();

fn store() -> &'static Mutex<HashMap<String, Vec<u8>>> {
    INTERNET_FS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register and resolve `http:` / `https:` stub URLs (`wxInternetFSHandler`).
#[derive(Debug, Default, Clone, Copy)]
pub struct InternetFSHandler;

impl InternetFSHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn normalize_url(url: &str) -> String {
        if url.starts_with("http://") || url.starts_with("https://") {
            url.to_string()
        } else {
            format!("https://{url}")
        }
    }

    pub fn register_stub(&self, url: &str, data: Vec<u8>) {
        let key = Self::normalize_url(url);
        if let Ok(mut map) = store().lock() {
            map.insert(key, data);
        }
    }

    pub fn register_text_stub(&self, url: &str, text: &str) {
        self.register_stub(url, text.as_bytes().to_vec());
    }

    pub fn fetch_stub(&self, url: &str) -> Option<Vec<u8>> {
        let key = Self::normalize_url(url);
        store().lock().ok()?.get(&key).cloned()
    }
}
