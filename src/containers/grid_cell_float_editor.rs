//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Grid float cell editor (`wxGridCellFloatEditor`).

use super::grid_cell_editor::GridCellEditor;

/// Floating-point editor for grid cells (`wxGridCellFloatEditor`).
#[derive(Debug, Clone)]
pub struct GridCellFloatEditor {
    inner: GridCellEditor,
}

impl GridCellFloatEditor {
    pub fn new(row: usize, col: usize) -> Self {
        Self {
            inner: GridCellEditor::new(row, col),
        }
    }

    pub fn begin_edit(&mut self, initial: f64) {
        self.inner.begin_edit(&initial.to_string());
    }

    pub fn end_edit(&mut self, accept: bool) -> Option<f64> {
        self.inner
            .end_edit(accept)
            .and_then(|s| s.parse().ok())
    }
}
