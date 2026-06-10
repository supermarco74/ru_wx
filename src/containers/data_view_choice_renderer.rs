//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Data-view choice renderer (`wxDataViewChoiceRenderer`).

use super::data_view::DataViewRenderer;

/// Renders one of several choices (`wxDataViewChoiceRenderer`).
#[derive(Debug, Clone)]
pub struct DataViewChoiceRenderer {
    choices: Vec<String>,
}

impl DataViewChoiceRenderer {
    pub fn new(choices: Vec<String>) -> Self {
        Self { choices }
    }

    pub fn label(&self, index: usize) -> Option<&str> {
        self.choices.get(index).map(String::as_str)
    }
}

impl DataViewRenderer for DataViewChoiceRenderer {
    fn render_text(&self, value: &str) -> String {
        value
            .parse::<usize>()
            .ok()
            .and_then(|i| self.label(i).map(str::to_string))
            .unwrap_or_else(|| value.to_string())
    }
}
