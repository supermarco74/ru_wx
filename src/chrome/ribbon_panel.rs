//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Ribbon panel group (`wxRibbonPanel`).

use super::ribbon_button_bar::RibbonButtonBar;

/// Group of tools on a ribbon page (`wxRibbonPanel`).
#[derive(Debug, Clone, Default)]
pub struct RibbonPanel {
    pub label: String,
    pub min_size: (i32, i32),
    bars: Vec<RibbonButtonBar>,
}

impl RibbonPanel {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            min_size: (80, 72),
            bars: Vec::new(),
        }
    }

    pub fn with_min_size(mut self, width: i32, height: i32) -> Self {
        self.min_size = (width, height);
        self
    }

    pub fn add_bar(&mut self, bar: RibbonButtonBar) {
        self.bars.push(bar);
    }

    pub fn bar_count(&self) -> usize {
        self.bars.len()
    }

    pub fn bars(&self) -> &[RibbonButtonBar] {
        &self.bars
    }
}
