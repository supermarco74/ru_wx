//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! AUI toolbar event (`wxAuiToolBarEvent`).

/// AUI toolbar gripper or tool action (`wxAuiToolBarEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuiToolBarEventKind {
    ToolClick,
    ToolDropdown,
    GripperClick,
    RightClick,
    BeginDrag,
    EndDrag,
}

/// Notification from a dockable AUI toolbar (`wxAuiToolBarEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuiToolBarEvent {
    pub kind: AuiToolBarEventKind,
    pub tool_id: u16,
}

impl AuiToolBarEvent {
    pub const fn new(kind: AuiToolBarEventKind, tool_id: u16) -> Self {
        Self { kind, tool_id }
    }
}
