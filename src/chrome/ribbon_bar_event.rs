//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Ribbon bar event (`wxRibbonBarEvent`).

/// Ribbon tab or tool action (`wxRibbonBarEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RibbonBarEventKind {
    TabChanged,
    ToolClick,
}

/// Notification from a ribbon bar (`wxRibbonBarEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RibbonBarEvent {
    pub kind: RibbonBarEventKind,
    pub tool_id: u16,
}

impl RibbonBarEvent {
    pub const fn new(kind: RibbonBarEventKind, tool_id: u16) -> Self {
        Self { kind, tool_id }
    }
}
