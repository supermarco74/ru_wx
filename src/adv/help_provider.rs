//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Context help provider (`wxHelpProvider`).

use std::collections::HashMap;

/// Maps control ids to help strings (`wxHelpProvider`).
#[derive(Debug, Default)]
pub struct HelpProvider {
    topics: HashMap<u32, String>,
    default_topic: String,
}

impl HelpProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_default_help(&mut self, text: &str) {
        self.default_topic = text.to_string();
    }

    pub fn add_help(&mut self, control_id: u32, text: &str) {
        self.topics.insert(control_id, text.to_string());
    }

    pub fn remove_help(&mut self, control_id: u32) -> bool {
        self.topics.remove(&control_id).is_some()
    }

    pub fn get_help(&self, control_id: u32) -> &str {
        self.topics
            .get(&control_id)
            .map(String::as_str)
            .unwrap_or(&self.default_topic)
    }

    pub fn has_help(&self, control_id: u32) -> bool {
        self.topics.contains_key(&control_id)
    }
}
