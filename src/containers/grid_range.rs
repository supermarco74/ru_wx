//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Grid cell range (`wxGridRange`).

use crate::containers::grid_coords::GridCoords;

/// Inclusive rectangular cell range (`wxGridRange`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridRange {
    pub top_left: GridCoords,
    pub bottom_right: GridCoords,
}

impl GridRange {
    pub const fn new(top_left: GridCoords, bottom_right: GridCoords) -> Self {
        Self {
            top_left,
            bottom_right,
        }
    }

    pub fn contains(&self, coords: GridCoords) -> bool {
        coords.row >= self.top_left.row
            && coords.row <= self.bottom_right.row
            && coords.col >= self.top_left.col
            && coords.col <= self.bottom_right.col
    }

    pub fn row_count(&self) -> usize {
        self.bottom_right.row - self.top_left.row + 1
    }

    pub fn col_count(&self) -> usize {
        self.bottom_right.col - self.top_left.col + 1
    }
}
