//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Grid date cell editor (`wxGridCellDateEditor`).

use super::grid_cell_editor::GridCellEditor;

/// Date editor for grid cells (`wxGridCellDateEditor`).
#[derive(Debug, Clone)]
pub struct GridCellDateEditor {
    inner: GridCellEditor,
}

impl GridCellDateEditor {
    pub fn new(row: usize, col: usize) -> Self {
        Self {
            inner: GridCellEditor::new(row, col),
        }
    }

    pub fn begin_edit(&mut self, iso_date: &str) {
        self.inner.begin_edit(iso_date);
    }

    pub fn end_edit(&mut self, accept: bool) -> Option<String> {
        self.inner.end_edit(accept)
    }
}
