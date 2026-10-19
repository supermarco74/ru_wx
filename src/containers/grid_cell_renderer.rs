//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Grid cell renderer (`wxGridCellRenderer`).

use crate::containers::grid::Cell;
use crate::core::geometry::Rect;

/// Live column renderer (`wxGridCellRenderer` used by [`crate::Grid`]).
pub trait GridColumnRenderer: Send + Sync {
    fn render_cell(&self, cell: &Cell) -> String;
}

/// Draws a grid cell (`wxGridCellRenderer`).
#[derive(Debug, Clone, Default)]
pub struct GridCellRenderer;

impl GridCellRenderer {
    pub fn new() -> Self {
        Self
    }

    pub fn render_text(&self, cell: &Cell, rect: Rect) -> String {
        let _ = rect;
        cell.display_text()
    }

    pub fn preferred_size(&self, cell: &Cell) -> (u32, u32) {
        let text = cell.display_text();
        let w = (text.len() as u32).saturating_mul(8).max(40);
        (w, 20)
    }
}

impl GridColumnRenderer for GridCellRenderer {
    fn render_cell(&self, cell: &Cell) -> String {
        self.render_text(cell, Rect::new(0, 0, 0, 0))
    }
}
