//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Application module (`wxModule`).

/// Pluggable init/shutdown unit (`wxModule`).
#[derive(Debug)]
pub struct WxModule {
    name: String,
    initialized: bool,
}

impl WxModule {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            initialized: false,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn initialize(&mut self) {
        self.initialized = true;
    }

    pub fn shutdown(&mut self) {
        self.initialized = false;
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}
