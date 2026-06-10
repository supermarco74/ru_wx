//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Grid boolean cell renderer (`wxGridCellBoolRenderer`).

/// Renders checkbox-style booleans (`wxGridCellBoolRenderer`).
#[derive(Debug, Clone, Copy, Default)]
pub struct GridCellBoolRenderer;

impl GridCellBoolRenderer {
    pub fn new() -> Self {
        Self
    }

    pub fn render_bool(&self, value: bool) -> String {
        if value { "☑" } else { "☐" }.to_string()
    }
}
