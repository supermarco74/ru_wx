//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! String grid table (`wxGridStringTable`).

use crate::containers::grid::Cell;
use crate::containers::grid_table::GridTable;

/// Table storing plain strings (`wxGridStringTable`).
#[derive(Debug, Clone, Default)]
pub struct GridStringTable {
    rows: Vec<Vec<String>>,
}

impl GridStringTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn resize(&mut self, rows: usize, cols: usize) {
        self.rows.resize_with(rows, || vec![String::new(); cols]);
        for row in &mut self.rows {
            row.resize(cols, String::new());
        }
    }

    pub fn set_value(&mut self, row: usize, col: usize, value: &str) {
        if row < self.rows.len() && col < self.rows[row].len() {
            self.rows[row][col] = value.to_string();
        }
    }

    pub fn get_value(&self, row: usize, col: usize) -> Option<&str> {
        self.rows.get(row)?.get(col).map(String::as_str)
    }
}

impl GridTable for GridStringTable {
    fn row_count(&self) -> usize {
        self.rows.len()
    }

    fn col_count(&self) -> usize {
        self.rows.first().map(|r| r.len()).unwrap_or(0)
    }

    fn value(&self, row: usize, col: usize) -> Cell {
        self.get_value(row, col)
            .map(|s| Cell::Text(s.to_string()))
            .unwrap_or(Cell::Empty)
    }

    fn set_value(&mut self, row: usize, col: usize, value: Cell) -> bool {
        if let Cell::Text(text) = value {
            self.set_value(row, col, &text);
            true
        } else {
            false
        }
    }
}
