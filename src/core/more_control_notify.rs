//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Additional control notify events (`wxRadioBoxEvent`, …).

/// Push button clicked (`wxButtonEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonEvent {
    pub command_id: u16,
}

impl ButtonEvent {
    pub const fn new(command_id: u16) -> Self {
        Self { command_id }
    }
}

/// Radio box selection changed (`wxRadioBoxEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadioBoxEvent {
    pub selection: usize,
}

impl RadioBoxEvent {
    pub const fn new(selection: usize) -> Self {
        Self { selection }
    }
}

/// Toggle button state changed (`wxToggleButtonEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToggleButtonEvent {
    pub pressed: bool,
}

impl ToggleButtonEvent {
    pub const fn new(pressed: bool) -> Self {
        Self { pressed }
    }
}

/// List box selection changed (`wxListBoxEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListBoxEvent {
    pub selection: usize,
    pub double_click: bool,
}

impl ListBoxEvent {
    pub const fn new(selection: usize) -> Self {
        Self {
            selection,
            double_click: false,
        }
    }

    pub const fn double_click(selection: usize) -> Self {
        Self {
            selection,
            double_click: true,
        }
    }
}

/// Choice selection changed (`wxChoiceEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChoiceEvent {
    pub selection: usize,
}

impl ChoiceEvent {
    pub const fn new(selection: usize) -> Self {
        Self { selection }
    }
}

/// Search field text changed (`wxSearchCtrlEvent`).
#[derive(Debug, Clone)]
pub struct SearchCtrlEvent {
    pub text: String,
    pub cancel_pressed: bool,
}

impl SearchCtrlEvent {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            cancel_pressed: false,
        }
    }

    pub fn cancel() -> Self {
        Self {
            text: String::new(),
            cancel_pressed: true,
        }
    }
}

