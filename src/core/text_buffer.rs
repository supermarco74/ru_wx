//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Mutable text buffer (`wxTextBuffer`).

/// Growable UTF-8 text buffer (`wxTextBuffer`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextBuffer {
    text: String,
}

impl TextBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a buffer holding a copy of `text`.
    // Infallible constructor mirroring wxTextBuffer usage; not the
    // `std::str::FromStr` trait (which would require an `Err`).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(text: &str) -> Self {
        Self {
            text: text.to_string(),
        }
    }

    pub fn clear(&mut self) {
        self.text.clear();
    }

    pub fn append(&mut self, fragment: &str) {
        self.text.push_str(fragment);
    }

    pub fn insert(&mut self, pos: usize, fragment: &str) {
        if pos <= self.text.len() {
            self.text.insert_str(pos, fragment);
        }
    }

    pub fn remove(&mut self, start: usize, len: usize) {
        if start < self.text.len() {
            let end = (start + len).min(self.text.len());
            self.text.replace_range(start..end, "");
        }
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn into_string(self) -> String {
        self.text
    }
}
