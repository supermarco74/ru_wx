//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Rich text style collection (`wxRichTextStyleSheet`).

use std::collections::HashMap;

use crate::adv::rich_text_style::RichTextStyle;

/// Named rich-text styles (`wxRichTextStyleSheet`).
#[derive(Debug, Clone, Default)]
pub struct RichTextStyleSheet {
    styles: HashMap<String, RichTextStyle>,
}

impl RichTextStyleSheet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_style(&mut self, style: RichTextStyle) {
        self.styles.insert(style.name.clone(), style);
    }

    pub fn get_style(&self, name: &str) -> Option<&RichTextStyle> {
        self.styles.get(name)
    }

    pub fn remove_style(&mut self, name: &str) -> bool {
        self.styles.remove(name).is_some()
    }

    pub fn style_names(&self) -> Vec<&str> {
        self.styles.keys().map(String::as_str).collect()
    }
}
