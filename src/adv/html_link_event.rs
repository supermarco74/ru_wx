//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! HTML link click event (`wxHtmlLinkEvent`).

/// Hyperlink clicked inside an HTML window (`wxHtmlLinkEvent`).
#[derive(Debug, Clone)]
pub struct HtmlLinkEvent {
    pub href: String,
    pub link_text: String,
}

impl HtmlLinkEvent {
    pub fn new(href: impl Into<String>, link_text: impl Into<String>) -> Self {
        Self {
            href: href.into(),
            link_text: link_text.into(),
        }
    }
}
