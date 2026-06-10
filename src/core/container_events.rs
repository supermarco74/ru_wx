//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Container widget events (`wxTreeEvent`, `wxListEvent`, …).

/// Tree control notification (`wxTreeEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeEventKind {
    SelectionChanged,
    ItemExpanded,
    ItemCollapsed,
    ItemActivated,
}

#[derive(Debug, Clone, Copy)]
pub struct TreeEvent {
    pub kind: TreeEventKind,
    pub item_id: isize,
}

impl TreeEvent {
    pub const fn new(kind: TreeEventKind, item_id: isize) -> Self {
        Self { kind, item_id }
    }
}

/// List control notification (`wxListEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListEventKind {
    ItemSelected,
    ItemDeselected,
    ItemActivated,
    ColumnClick,
}

#[derive(Debug, Clone, Copy)]
pub struct ListEvent {
    pub kind: ListEventKind,
    pub item_index: i32,
    pub column: i32,
}

impl ListEvent {
    pub const fn new(kind: ListEventKind, item_index: i32, column: i32) -> Self {
        Self {
            kind,
            item_index,
            column,
        }
    }
}

/// Grid cell notification (`wxGridEvent`).
#[derive(Debug, Clone, Copy)]
pub struct GridEvent {
    pub row: i32,
    pub col: i32,
    pub selecting: bool,
}

impl GridEvent {
    pub const fn new(row: i32, col: i32, selecting: bool) -> Self {
        Self { row, col, selecting }
    }
}

/// Item double-click / enter (`wxItemActivateEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemActivateEvent {
    pub index: i32,
}

impl ItemActivateEvent {
    pub const fn new(index: i32) -> Self {
        Self { index }
    }
}
