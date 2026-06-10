//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Grid cell editor (`wxGridCellEditor`).

/// Edits a single grid cell (`wxGridCellEditor`).
#[derive(Debug, Clone, Default)]
pub struct GridCellEditor {
    pub row: usize,
    pub col: usize,
    pub text: String,
    pub active: bool,
}

impl GridCellEditor {
    pub fn new(row: usize, col: usize) -> Self {
        Self {
            row,
            col,
            text: String::new(),
            active: false,
        }
    }

    pub fn begin_edit(&mut self, initial: &str) {
        self.text = initial.to_string();
        self.active = true;
    }

    pub fn end_edit(&mut self, accept: bool) -> Option<String> {
        self.active = false;
        if accept {
            Some(self.text.clone())
        } else {
            None
        }
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
    }
}
