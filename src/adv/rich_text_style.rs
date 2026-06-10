//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Rich text paragraph style (`wxRichTextStyle`).

use crate::adv::rich_text_attr::RichTextAttr;

/// Paragraph-level rich text style (`wxRichTextStyle`).
#[derive(Debug, Clone, Default)]
pub struct RichTextStyle {
    pub name: String,
    pub attr: RichTextAttr,
    pub left_indent: i32,
    pub right_indent: i32,
}

impl RichTextStyle {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            attr: RichTextAttr::new(),
            left_indent: 0,
            right_indent: 0,
        }
    }

    pub fn with_attr(mut self, attr: RichTextAttr) -> Self {
        self.attr = attr;
        self
    }

    pub fn with_indent(mut self, left: i32, right: i32) -> Self {
        self.left_indent = left;
        self.right_indent = right;
        self
    }
}
