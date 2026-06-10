//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Single-topic help provider (`wxSimpleHelpProvider`).

use crate::adv::help_provider::HelpProvider;

/// Help provider backed by one default string (`wxSimpleHelpProvider`).
#[derive(Debug, Default)]
pub struct SimpleHelpProvider {
    inner: HelpProvider,
    window_topic: String,
}

impl SimpleHelpProvider {
    pub fn new(topic: &str) -> Self {
        let mut inner = HelpProvider::new();
        inner.set_default_help(topic);
        Self {
            inner,
            window_topic: topic.to_string(),
        }
    }

    pub fn topic(&self) -> &str {
        &self.window_topic
    }

    pub fn set_topic(&mut self, topic: &str) {
        self.window_topic = topic.to_string();
        self.inner.set_default_help(topic);
    }

    pub fn add_control_help(&mut self, control_id: u32, text: &str) {
        self.inner.add_help(control_id, text);
    }

    pub fn get_help(&self, control_id: u32) -> &str {
        self.inner.get_help(control_id)
    }

    pub fn provider(&self) -> &HelpProvider {
        &self.inner
    }
}
