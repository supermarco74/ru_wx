//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Hyperlink click event (`wxHyperlinkEvent`).

/// Link activated (`wxHyperlinkEvent`).
#[derive(Debug, Clone)]
pub struct HyperlinkEvent {
    pub url: String,
}

impl HyperlinkEvent {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}
