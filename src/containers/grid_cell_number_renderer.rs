//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Grid number cell renderer (`wxGridCellNumberRenderer`).

use crate::containers::grid::{Cell, NumberFormat};

use super::grid_cell_renderer::GridCellRenderer;

/// Renders numeric cells (`wxGridCellNumberRenderer`).
#[derive(Debug, Clone, Copy)]
pub struct GridCellNumberRenderer {
    format: NumberFormat,
}

impl Default for GridCellNumberRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl GridCellNumberRenderer {
    pub fn new() -> Self {
        Self {
            format: NumberFormat::Plain,
        }
    }

    pub fn with_format(format: NumberFormat) -> Self {
        Self { format }
    }

    pub fn render_number(&self, value: f64) -> String {
        let cell = Cell::Number {
            value,
            format: self.format,
        };
        GridCellRenderer::new().render_text(&cell, crate::core::geometry::Rect::new(0, 0, 0, 0))
    }
}
