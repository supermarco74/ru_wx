//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Data-view toggle renderer (`wxDataViewToggleRenderer`).

use super::data_view::DataViewRenderer;

/// Renders on/off toggles (`wxDataViewToggleRenderer`).
#[derive(Debug, Clone, Copy, Default)]
pub struct DataViewToggleRenderer;

impl DataViewToggleRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl DataViewRenderer for DataViewToggleRenderer {
    fn render_text(&self, value: &str) -> String {
        match value {
            "1" | "true" | "yes" => "☑".to_string(),
            _ => "☐".to_string(),
        }
    }
}
