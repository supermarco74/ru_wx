//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Grid boolean cell editor (`wxGridCellBoolEditor`).

/// Toggle editor for grid cells (`wxGridCellBoolEditor`).
#[derive(Debug, Clone, Default)]
pub struct GridCellBoolEditor {
    pub row: usize,
    pub col: usize,
    pub value: bool,
}

impl GridCellBoolEditor {
    pub fn new(row: usize, col: usize) -> Self {
        Self {
            row,
            col,
            value: false,
        }
    }

    pub fn toggle(&mut self) {
        self.value = !self.value;
    }
}
