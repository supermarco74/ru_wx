//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Reorder list dialog (`wxRearrangeDialog`).

use crate::window::frame::Frame;

/// Modal dialog to reorder a list of strings (`wxRearrangeDialog`).
pub struct RearrangeDialog {
    title: String,
    items: Vec<String>,
}

impl RearrangeDialog {
    pub fn new(title: &str, items: Vec<String>) -> Self {
        Self {
            title: title.to_string(),
            items,
        }
    }

    pub fn items(&self) -> &[String] {
        &self.items
    }

    pub fn move_up(&mut self, index: usize) {
        if index > 0 && index < self.items.len() {
            self.items.swap(index, index - 1);
        }
    }

    pub fn move_down(&mut self, index: usize) {
        if index + 1 < self.items.len() {
            self.items.swap(index, index + 1);
        }
    }

    /// Show modally. Returns reordered items, or `None` if cancelled.
    pub fn show_modal(self, _frame: &Frame) -> Option<Vec<String>> {
        // Stub: returns items unchanged (full Win32 UI is a follow-up).
        let _ = self.title;
        Some(self.items)
    }
}
