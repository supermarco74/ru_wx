//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Rich text control event (`wxRichTextEvent`).

/// Rich-text buffer changed (`wxRichTextEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RichTextEventKind {
    TextUpdated,
    SelectionChanged,
    StyleChanged,
}

/// Notification from a rich-text control (`wxRichTextEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RichTextEvent {
    pub kind: RichTextEventKind,
    pub position: usize,
    pub length: usize,
}

impl RichTextEvent {
    pub const fn new(kind: RichTextEventKind, position: usize, length: usize) -> Self {
        Self {
            kind,
            position,
            length,
        }
    }
}
