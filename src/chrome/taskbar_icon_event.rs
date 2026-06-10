//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Task-bar icon events (`wxTaskBarIconEvent`).

/// Tray icon interaction (`wxTaskBarIconEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskBarIconEventKind {
    LeftClick,
    RightClick,
    DoubleClick,
    BalloonClick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskBarIconEvent {
    pub kind: TaskBarIconEventKind,
}

impl TaskBarIconEvent {
    pub const fn new(kind: TaskBarIconEventKind) -> Self {
        Self { kind }
    }
}
