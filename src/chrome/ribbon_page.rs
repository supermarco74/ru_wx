//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Ribbon tab page (`wxRibbonPage`).

use super::ribbon_panel::RibbonPanel;

/// Ribbon tab context type (per Microsoft Fluent UX).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RibbonContextType {
    #[default]
    None,
    TextSelected,
    ImageSelected,
    TableSelected,
    DrawingSelected,
}

/// One tab page in a [`super::ribbon_bar::RibbonBar`] (`wxRibbonPage`).
#[derive(Debug, Clone, Default)]
pub struct RibbonPage {
    pub label: String,
    /// Whether this is a contextual tab (hidden until context is active).
    pub is_contextual: bool,
    /// Context type that activates this tab.
    pub context_type: RibbonContextType,
    /// Context color for contextual tabs (e.g. pink for Image, green for Table).
    pub context_color: (u8, u8, u8),
    panels: Vec<RibbonPanel>,
}

impl RibbonPage {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            is_contextual: false,
            context_type: RibbonContextType::None,
            context_color: (0, 0, 0),
            panels: Vec::new(),
        }
    }

    /// Create a contextual tab that appears only when the given context is active.
    pub fn new_contextual(
        label: &str,
        context_type: RibbonContextType,
        context_color: (u8, u8, u8),
    ) -> Self {
        Self {
            label: label.to_string(),
            is_contextual: true,
            context_type,
            context_color,
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
