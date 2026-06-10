//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Ribbon gallery (`wxRibbonGallery`).

/// Scrollable ribbon icon gallery (`wxRibbonGallery`).
#[derive(Debug, Clone, Default)]
pub struct RibbonGallery {
    items: Vec<String>,
    selection: usize,
}

impl RibbonGallery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&mut self, label: &str) {
        self.items.push(label.to_string());
    }

    pub fn selection(&self) -> Option<usize> {
        if self.items.is_empty() {
            None
        } else {
            Some(self.selection.min(self.items.len() - 1))
        }
    }

    pub fn set_selection(&mut self, index: usize) {
        if index < self.items.len() {
            self.selection = index;
        }
    }

    pub fn selected_label(&self) -> Option<&str> {
        self.selection().and_then(|i| self.items.get(i).map(String::as_str))
    }

    pub fn items(&self) -> &[String] {
        &self.items
    }
}
