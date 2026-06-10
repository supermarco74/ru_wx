//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Environment variables (`wxGetEnv` / `wxSetEnv`).

use std::env;

/// Process environment access (`wxGetEnv` / `wxSetEnv`).
#[derive(Debug, Default, Clone, Copy)]
pub struct Environment;

impl Environment {
    pub fn new() -> Self {
        Self
    }

    pub fn get_var(name: &str) -> Option<String> {
        env::var(name).ok()
    }

    pub fn set_var(name: &str, value: &str) -> bool {
        env::set_var(name, value);
        true
    }

    pub fn has_var(name: &str) -> bool {
        env::var_os(name).is_some()
    }

    pub fn remove_var(name: &str) -> bool {
        env::remove_var(name);
        true
    }
}
