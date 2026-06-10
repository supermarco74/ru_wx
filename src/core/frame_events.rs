//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Additional frame events (`wxIconizeEvent`, `wxMaximizeEvent`, …).

/// Window minimized (`wxIconizeEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconizeEvent {
    pub iconized: bool,
}

impl IconizeEvent {
    pub const fn iconized() -> Self {
        Self { iconized: true }
    }

    pub const fn restored() -> Self {
        Self { iconized: false }
    }
}

/// Window maximized (`wxMaximizeEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaximizeEvent {
    pub maximized: bool,
}

impl MaximizeEvent {
    pub const fn maximized() -> Self {
        Self { maximized: true }
    }

    pub const fn restored() -> Self {
        Self { maximized: false }
    }
}

/// Help button / F1 (`wxHelpEvent`).
#[derive(Debug, Clone, Copy, Default)]
pub struct HelpEvent {
    pub control_id: u16,
}

impl HelpEvent {
    pub const fn new(control_id: u16) -> Self {
        Self { control_id }
    }
}

/// Menu/toolbar UI refresh (`wxUpdateUIEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateUIEvent {
    pub id: u16,
}

impl UpdateUIEvent {
    pub const fn new(id: u16) -> Self {
        Self { id }
    }
}

/// Tab / focus navigation (`wxNavigationKeyEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavigationKeyEvent {
    pub forward: bool,
    pub from_tab: bool,
}

impl NavigationKeyEvent {
    pub const fn new(forward: bool, from_tab: bool) -> Self {
        Self { forward, from_tab }
    }
}

/// Battery / suspend (`wxPowerEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerEventKind {
    Suspend,
    Resume,
    BatteryLow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerEvent {
    pub kind: PowerEventKind,
}

impl PowerEvent {
    pub const fn new(kind: PowerEventKind) -> Self {
        Self { kind }
    }
}
