//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Ribbon button bar (`wxRibbonButtonBar`).

/// Horizontal strip of ribbon tools (`wxRibbonButtonBar`).
#[derive(Debug, Clone, Default)]
pub struct RibbonButtonBar {
    buttons: Vec<(u16, String)>,
}

impl RibbonButtonBar {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_button(&mut self, id: u16, label: &str) {
        self.buttons.push((id, label.to_string()));
    }

    pub fn button_count(&self) -> usize {
        self.buttons.len()
    }

    pub fn buttons(&self) -> &[(u16, String)] {
        &self.buttons
    }
}
