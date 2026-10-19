//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Grid data model (`wxGridTableBase`).

use crate::containers::grid::Cell;

/// Tabular data source for [`crate::Grid`] (`wxGridTableBase`).
pub trait GridTable {
    fn row_count(&self) -> usize;
    fn col_count(&self) -> usize;
    fn value(&self, row: usize, col: usize) -> Cell;
    fn set_value(&mut self, row: usize, col: usize, value: Cell) -> bool;
}

/// In-memory grid table backed by a closure.
pub struct FunctionGridTable<F>
where
    F: Fn(usize, usize) -> Cell,
{
    rows: usize,
    cols: usize,
    provider: F,
}

impl<F> FunctionGridTable<F>
where
    F: Fn(usize, usize) -> Cell,
{
    pub fn new(rows: usize, cols: usize, provider: F) -> Self {
        Self {
            rows,
            cols,
            provider,
        }
    }

    /// Alias for [`Self::new`] emphasising the read-only nature of the
    /// resulting table (the provider has no setter side; `set_value`
    /// always returns `false`). Matches the `new_no_setter()` name used
    /// by examples and matches `wxGridTableBase::SetValue` returning
    /// `false` for read-only backends.
    pub fn new_no_setter(rows: usize, cols: usize, provider: F) -> Self {
        Self::new(rows, cols, provider)
    }
}

impl<F> GridTable for FunctionGridTable<F>
where
    F: Fn(usize, usize) -> Cell,
{
    fn row_count(&self) -> usize {
        self.rows
    }

    fn col_count(&self) -> usize {
        self.cols
    }

    fn value(&self, row: usize, col: usize) -> Cell {
        (self.provider)(row, col)
    }

    fn set_value(&mut self, _row: usize, _col: usize, _value: Cell) -> bool {
        false
    }
}
