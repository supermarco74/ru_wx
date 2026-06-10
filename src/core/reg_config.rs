//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Registry config (`wxRegConfig`) — in-memory stub on all platforms.

use std::collections::HashMap;

/// Registry hive placeholder.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum RegRoot {
    #[default]
    CurrentUser,
    LocalMachine,
}

/// Registry-backed settings (`wxRegConfig`).
#[derive(Debug)]
pub struct RegConfig {
    root: RegRoot,
    path: String,
    values: HashMap<String, String>,
}

impl RegConfig {
    pub fn new(root: RegRoot, path: &str) -> Self {
        Self {
            root,
            path: path.to_string(),
            values: HashMap::new(),
        }
    }

    pub fn read(&self, name: &str, default: &str) -> String {
        self.values
            .get(name)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }

    pub fn write(&mut self, name: &str, value: &str) {
        self.values.insert(name.to_string(), value.to_string());
    }

    pub fn root(&self) -> RegRoot {
        self.root
    }

    pub fn key_path(&self) -> &str {
        &self.path
    }
}
