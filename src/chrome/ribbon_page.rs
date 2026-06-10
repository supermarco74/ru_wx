//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Ribbon tab page (`wxRibbonPage`).

use super::ribbon_panel::RibbonPanel;

/// One tab page in a [`super::ribbon_bar::RibbonBar`] (`wxRibbonPage`).
#[derive(Debug, Clone, Default)]
pub struct RibbonPage {
    pub label: String,
    panels: Vec<RibbonPanel>,
}

impl RibbonPage {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            panels: Vec::new(),
        }
    }

    pub fn add_panel(&mut self, panel: RibbonPanel) {
        self.panels.push(panel);
    }

    pub fn panel_count(&self) -> usize {
        self.panels.len()
    }

    pub fn panel(&self, index: usize) -> Option<&RibbonPanel> {
        self.panels.get(index)
    }

    pub fn panels(&self) -> &[RibbonPanel] {
        &self.panels
    }
}
