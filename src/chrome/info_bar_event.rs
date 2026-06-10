//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Info bar event (`wxInfoBarEvent`).

use crate::chrome::info_bar::InfoBarMessageType;

/// Info bar message or dismiss (`wxInfoBarEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoBarEventKind {
    MessageShown,
    Dismissed,
}

/// Notification from an info bar (`wxInfoBarEvent`).
#[derive(Debug, Clone)]
pub struct InfoBarEvent {
    pub kind: InfoBarEventKind,
    pub message_type: InfoBarMessageType,
    pub text: String,
}

impl InfoBarEvent {
    pub fn new(kind: InfoBarEventKind, message_type: InfoBarMessageType, text: impl Into<String>) -> Self {
        Self {
            kind,
            message_type,
            text: text.into(),
        }
    }
}
