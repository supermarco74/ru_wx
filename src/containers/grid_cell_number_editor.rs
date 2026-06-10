//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Grid number cell editor (`wxGridCellNumberEditor`).

use super::grid_cell_editor::GridCellEditor;

/// Integer editor for grid cells (`wxGridCellNumberEditor`).
#[derive(Debug, Clone)]
pub struct GridCellNumberEditor {
    inner: GridCellEditor,
}

impl GridCellNumberEditor {
    pub fn new(row: usize, col: usize) -> Self {
        Self {
            inner: GridCellEditor::new(row, col),
        }
    }

    pub fn begin_edit(&mut self, initial: i32) {
        self.inner.begin_edit(&initial.to_string());
    }

    pub fn end_edit(&mut self, accept: bool) -> Option<i32> {
        self.inner
            .end_edit(accept)
            .and_then(|s| s.parse().ok())
    }
}
