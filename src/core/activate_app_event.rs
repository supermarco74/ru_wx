//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Application activation event (`wxActivateAppEvent`).

/// Application gained or lost activation (`wxActivateAppEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivateAppEvent {
    pub active: bool,
}

impl ActivateAppEvent {
    pub const fn activated() -> Self {
        Self { active: true }
    }

    pub const fn deactivated() -> Self {
        Self { active: false }
    }
}
