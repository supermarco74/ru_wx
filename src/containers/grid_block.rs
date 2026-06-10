//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Grid cell block (`wxGridBlock`).

use crate::containers::grid::Cell;
use crate::containers::grid_coords::GridCoords;
use crate::containers::grid_range::GridRange;

/// Rectangular block of homogeneous cells (`wxGridBlock`).
#[derive(Debug, Clone)]
pub struct GridBlock {
    pub range: GridRange,
    pub value: Cell,
}

impl GridBlock {
    pub fn new(range: GridRange, value: Cell) -> Self {
        Self { range, value }
    }

    pub fn contains(&self, coords: GridCoords) -> bool {
        self.range.contains(coords)
    }
}
