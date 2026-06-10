//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! System command event (`wxSysCommandEvent`).

/// System menu / chrome command (`wxSysCommandEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SysCommandEvent {
    pub command: u32,
}

impl SysCommandEvent {
    pub const fn new(command: u32) -> Self {
        Self { command }
    }

    pub const fn close() -> Self {
        Self::new(0xF060)
    }

    pub const fn maximize() -> Self {
        Self::new(0xF030)
    }

    pub const fn restore() -> Self {
        Self::new(0xF120)
    }
}
