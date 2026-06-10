//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Grid cell attributes (`wxGridCellAttr`).

use crate::containers::grid::GridCellStyle;
use crate::core::geometry::Colour;

/// Per-cell display attributes (`wxGridCellAttr`).
#[derive(Debug, Clone, Default)]
pub struct GridCellAttr {
    pub style: GridCellStyle,
    pub bg: Option<Colour>,
    pub fg: Option<Colour>,
    pub read_only: bool,
}

impl GridCellAttr {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_style(mut self, style: GridCellStyle) -> Self {
        self.style = style;
        self
    }

    pub fn with_background(mut self, colour: Colour) -> Self {
        self.bg = Some(colour);
        self
    }

    pub fn with_foreground(mut self, colour: Colour) -> Self {
        self.fg = Some(colour);
        self
    }

    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }
}
