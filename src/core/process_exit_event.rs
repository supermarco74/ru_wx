//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Process exit notification (`wxProcessExitEvent`).

/// Child process exited (`wxProcessExitEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessExitEvent {
    pub pid: u32,
    pub exit_code: i32,
}

impl ProcessExitEvent {
    pub const fn new(pid: u32, exit_code: i32) -> Self {
        Self { pid, exit_code }
    }
}
