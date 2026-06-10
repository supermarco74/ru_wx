//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Rich text formatting dialog (`wxRichTextFormattingDialog`).

use crate::adv::rich_text_attr::RichTextAttr;
use crate::window::frame::Frame;

/// Modal dialog for rich-text attributes (`wxRichTextFormattingDialog`).
pub struct RichTextFormattingDialog {
    title: String,
    attr: RichTextAttr,
}

impl RichTextFormattingDialog {
    pub fn new(title: &str, attr: RichTextAttr) -> Self {
        Self {
            title: title.to_string(),
            attr,
        }
    }

    pub fn attr(&self) -> &RichTextAttr {
        &self.attr
    }

    pub fn show_modal(self, _frame: &Frame) -> Option<RichTextAttr> {
        let _ = self.title;
        Some(self.attr)
    }
}
