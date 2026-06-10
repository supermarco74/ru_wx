//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Grid string cell renderer (`wxGridCellStringRenderer`).

use crate::containers::grid::Cell;

use super::grid_cell_renderer::GridCellRenderer;

/// Renders plain text cells (`wxGridCellStringRenderer`).
#[derive(Debug, Clone, Copy, Default)]
pub struct GridCellStringRenderer;

impl GridCellStringRenderer {
    pub fn new() -> Self {
        Self
    }

    pub fn render_string(&self, text: &str) -> String {
        let cell = Cell::Text(text.to_string());
        GridCellRenderer::new().render_text(&cell, crate::core::geometry::Rect::new(0, 0, 0, 0))
    }
}
