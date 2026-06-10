//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Rich text document buffer (`wxRichTextBuffer`).

/// Styled text storage (`wxRichTextBuffer`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RichTextBuffer {
    plain: String,
    bold_ranges: Vec<(usize, usize)>,
}

impl RichTextBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_plain(text: &str) -> Self {
        Self {
            plain: text.to_string(),
            bold_ranges: Vec::new(),
        }
    }

    pub fn plain_text(&self) -> &str {
        &self.plain
    }

    pub fn set_plain_text(&mut self, text: &str) {
        self.plain = text.to_string();
        self.bold_ranges.clear();
    }

    pub fn append_plain(&mut self, text: &str) {
        self.plain.push_str(text);
    }

    pub fn add_bold_range(&mut self, start: usize, end: usize) {
        if start < end && end <= self.plain.len() {
            self.bold_ranges.push((start, end));
        }
    }

    pub fn bold_ranges(&self) -> &[(usize, usize)] {
        &self.bold_ranges
    }
}
