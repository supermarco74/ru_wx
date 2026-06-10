//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Popup window event (`wxPopupWindowEvent`).

/// Popup shown or dismissed (`wxPopupWindowEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupWindowEventKind {
    Show,
    Dismiss,
}

/// Notification from a popup window (`wxPopupWindowEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopupWindowEvent {
    pub kind: PopupWindowEventKind,
}

impl PopupWindowEvent {
    pub const fn new(kind: PopupWindowEventKind) -> Self {
        Self { kind }
    }
}
