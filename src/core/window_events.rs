//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Additional window events (`wxIdleEvent`, `wxActivateEvent`, …).

/// Idle-time processing hook (`wxIdleEvent`).
#[derive(Debug, Clone, Copy, Default)]
pub struct IdleEvent {
    pub more_requested: bool,
}

impl IdleEvent {
    pub const fn new() -> Self {
        Self {
            more_requested: false,
        }
    }

    /// Ask for another idle pass (`RequestMore`).
    pub fn request_more(&mut self) {
        self.more_requested = true;
    }
}

/// Window activation (`wxActivateEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivateEvent {
    pub active: bool,
}

impl ActivateEvent {
    pub const fn activated() -> Self {
        Self { active: true }
    }

    pub const fn deactivated() -> Self {
        Self { active: false }
    }

    pub(crate) fn from_win32(wparam: usize) -> Self {
        let active = (wparam & 0xFFFF) != 0;
        Self { active }
    }
}

/// Character input (`wxCharEvent` / `wxCharHookEvent`).
#[derive(Debug, Clone, Copy)]
pub struct CharEvent {
    pub unicode: u32,
    pub is_hook: bool,
}

impl CharEvent {
    pub(crate) fn from_win32(code: u32, hook: bool) -> Self {
        Self {
            unicode: code,
            is_hook: hook,
        }
    }
}

/// Custom paint notification (`wxPaintEvent`).
#[derive(Debug, Clone, Copy)]
pub struct PaintEvent {
    pub hdc: isize,
}

impl PaintEvent {
    pub const fn new(hdc: isize) -> Self {
        Self { hdc }
    }
}

/// Background erase (`wxEraseEvent`).
#[derive(Debug)]
pub struct EraseEvent {
    vetoed: std::cell::Cell<bool>,
}

impl EraseEvent {
    pub(crate) fn new() -> Self {
        Self {
            vetoed: std::cell::Cell::new(false),
        }
    }

    pub fn veto(&self) {
        self.vetoed.set(true);
    }

    pub fn is_vetoed(&self) -> bool {
        self.vetoed.get()
    }
}

/// Window shown (`wxShowEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowEvent;

impl Default for ShowEvent {
    fn default() -> Self {
        Self::new()
    }
}

impl ShowEvent {
    pub const fn new() -> Self {
        Self
    }
}

/// Window hidden (`wxHideEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HideEvent;

impl Default for HideEvent {
    fn default() -> Self {
        Self::new()
    }
}

impl HideEvent {
    pub const fn new() -> Self {
        Self
    }
}

/// Generic notification base (`wxNotifyEvent`).
#[derive(Debug, Clone, Copy, Default)]
pub struct NotifyEvent {
    allowed: bool,
}

impl NotifyEvent {
    pub const fn new() -> Self {
        Self { allowed: true }
    }

    pub fn veto(&mut self) {
        self.allowed = false;
    }

    pub fn is_allowed(&self) -> bool {
        self.allowed
    }
}
