//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Rich text attributes (`wxRichTextAttr`).

use crate::core::geometry::Colour;

/// Character formatting for rich text (`wxRichTextAttr`).
#[derive(Debug, Clone, Default)]
pub struct RichTextAttr {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub font_size: u32,
    pub text_colour: Option<Colour>,
    pub background_colour: Option<Colour>,
}

impl RichTextAttr {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    pub fn with_font_size(mut self, size: u32) -> Self {
        self.font_size = size;
        self
    }

    pub fn with_text_colour(mut self, colour: Colour) -> Self {
        self.text_colour = Some(colour);
        self
    }
}
