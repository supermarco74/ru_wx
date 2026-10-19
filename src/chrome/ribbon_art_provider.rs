//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Ribbon theme colours (`wxRibbonArtProvider`).

use crate::core::geometry::Colour;

/// Ribbon colour scheme (`wxRibbonArtProvider`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RibbonArtProvider {
    pub tab_background: Colour,
    pub panel_background: Colour,
    pub accent: Colour,
}

impl Default for RibbonArtProvider {
    fn default() -> Self {
        Self {
            tab_background: Colour::new(240, 240, 240, 255),
            panel_background: Colour::new(252, 252, 252, 255),
            accent: crate::core::tokens::system_accent_colour(),
        }
    }
}

impl RibbonArtProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_accent(mut self, colour: Colour) -> Self {
        self.accent = colour;
        self
    }
}
