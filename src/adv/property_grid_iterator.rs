//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Property grid iterator (`wxPropertyGridIterator`).

use crate::adv::property_grid::{PropertyGrid, PropertyValue};

/// Walks properties in a [`PropertyGrid`] (`wxPropertyGridIterator`).
pub struct PropertyGridIterator<'a> {
    grid: &'a PropertyGrid,
    index: usize,
}

impl<'a> PropertyGridIterator<'a> {
    pub fn new(grid: &'a PropertyGrid) -> Self {
        Self { grid, index: 0 }
    }
}

impl<'a> Iterator for PropertyGridIterator<'a> {
    type Item = (usize, String, PropertyValue);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.grid.len() {
            return None;
        }
        let idx = self.index;
        let name = self.grid.get_name(idx)?;
        let value = self.grid.get_value(idx)?;
        self.index += 1;
        Some((idx, name, value))
    }
}
