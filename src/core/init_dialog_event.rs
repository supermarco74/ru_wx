//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Dialog init event (`wxInitDialogEvent`).

/// Sent when a dialog is about to be shown (`wxInitDialogEvent`).
#[derive(Debug, Clone, Copy, Default)]
pub struct InitDialogEvent;

impl InitDialogEvent {
    pub const fn new() -> Self {
        Self
    }
}
