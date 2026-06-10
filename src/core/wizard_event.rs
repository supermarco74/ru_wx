//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Wizard navigation event (`wxWizardEvent`).

/// Wizard page or button action (`wxWizardEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardEventKind {
    PageChanged,
    PageChanging,
    Cancelled,
    Finished,
}

/// Wizard navigation notification (`wxWizardEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WizardEvent {
    pub kind: WizardEventKind,
    pub page_index: usize,
}

impl WizardEvent {
    pub const fn new(kind: WizardEventKind, page_index: usize) -> Self {
        Self { kind, page_index }
    }
}

