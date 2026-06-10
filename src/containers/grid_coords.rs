//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Grid coordinates (`wxGridCoords`).

/// Row/column address in a grid (`wxGridCoords`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridCoords {
    pub row: usize,
    pub col: usize,
}

impl GridCoords {
    pub const fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }

    pub fn is_valid(&self, rows: usize, cols: usize) -> bool {
        self.row < rows && self.col < cols
    }
}
