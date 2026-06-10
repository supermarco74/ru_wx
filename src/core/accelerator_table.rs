//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Accelerator table (`wxAcceleratorTable` / `wxAcceleratorEntry`).

use crate::core::accelerator::Accelerator;

/// Single shortcut binding (`wxAcceleratorEntry`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceleratorEntry {
    pub accel: Accelerator,
    pub command_id: u16,
}

impl AcceleratorEntry {
    pub fn new(accel: Accelerator, command_id: u16) -> Self {
        Self { accel, command_id }
    }
}

/// Collection of shortcuts (`wxAcceleratorTable`).
#[derive(Debug, Clone, Default)]
pub struct AcceleratorTable {
    entries: Vec<AcceleratorEntry>,
}

impl AcceleratorTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, entry: AcceleratorEntry) {
        self.entries.push(entry);
    }

    pub fn entries(&self) -> &[AcceleratorEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
