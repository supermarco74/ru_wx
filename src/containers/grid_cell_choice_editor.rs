//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Grid choice cell editor (`wxGridCellChoiceEditor`).

/// Drop-down editor for grid cells (`wxGridCellChoiceEditor`).
#[derive(Debug, Clone)]
pub struct GridCellChoiceEditor {
    pub row: usize,
    pub col: usize,
    pub choices: Vec<String>,
    pub selection: usize,
}

impl GridCellChoiceEditor {
    pub fn new(row: usize, col: usize, choices: Vec<String>) -> Self {
        Self {
            row,
            col,
            choices,
            selection: 0,
        }
    }

    pub fn selected(&self) -> Option<&str> {
        self.choices.get(self.selection).map(String::as_str)
    }

    pub fn set_selection(&mut self, index: usize) {
        if index < self.choices.len() {
            self.selection = index;
        }
    }
}
