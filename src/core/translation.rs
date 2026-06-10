//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Message catalog (`wxTranslation` / `wxGetTranslation`).

use std::collections::HashMap;

/// In-memory translation table (`wxTranslation`).
#[derive(Debug, Default, Clone)]
pub struct Translation {
    strings: HashMap<String, String>,
    language: String,
}

impl Translation {
    pub fn new(language: &str) -> Self {
        Self {
            strings: HashMap::new(),
            language: language.to_string(),
        }
    }

    pub fn language(&self) -> &str {
        &self.language
    }

    pub fn add(&mut self, source: &str, translated: &str) {
        self.strings
            .insert(source.to_string(), translated.to_string());
    }

    pub fn get(&self, source: &str) -> String {
        self.strings
            .get(source)
            .cloned()
            .unwrap_or_else(|| source.to_string())
    }

    pub fn contains(&self, source: &str) -> bool {
        self.strings.contains_key(source)
    }
}

/// Translate `source` using the active catalog, or return `source`.
pub fn get_translation(catalog: &Translation, source: &str) -> String {
    catalog.get(source)
}
