//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Text/check/combo notifications (`wxTextEvent`, …).
//!
//! For drop-down list controls use [`crate::ChoiceEvent`] on [`crate::Choice`];
//! [`ComboBoxEvent`] here targets editable [`crate::ComboBox`] instances.

/// Text control changed (`wxTextEvent`).
#[derive(Debug, Clone)]
pub struct TextEvent {
    pub value: String,
}

impl TextEvent {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }
}

/// Editable combo selection changed (`wxComboBoxEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComboBoxEvent {
    pub selection: usize,
}

impl ComboBoxEvent {
    pub const fn new(selection: usize) -> Self {
        Self { selection }
    }
}

/// Checkbox toggled (`wxCheckBoxEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckBoxEvent {
    pub checked: bool,
}

impl CheckBoxEvent {
    pub const fn new(checked: bool) -> Self {
        Self { checked }
    }
}
