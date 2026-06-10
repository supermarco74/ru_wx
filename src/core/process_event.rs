//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Process termination events (`wxProcessEvent`).

/// Child process state change (`wxProcessEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessEventKind {
    Terminate,
    Error,
}

#[derive(Debug, Clone, Copy)]
pub struct ProcessEvent {
    pub kind: ProcessEventKind,
    pub exit_code: i32,
}

impl ProcessEvent {
    pub const fn terminate(exit_code: i32) -> Self {
        Self {
            kind: ProcessEventKind::Terminate,
            exit_code,
        }
    }

    pub const fn error(exit_code: i32) -> Self {
        Self {
            kind: ProcessEventKind::Error,
            exit_code,
        }
    }
}
