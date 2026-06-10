//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Data view control events (`wxDataViewEvent`).

/// Data view selection or edit (`wxDataViewEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataViewEventKind {
    SelectionChanged,
    ItemActivated,
    ItemExpanded,
    ItemCollapsed,
    ItemStartEditing,
    ItemEditingDone,
}

/// Notification from a data view (`wxDataViewEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataViewEvent {
    pub kind: DataViewEventKind,
    pub item_index: usize,
    pub column: usize,
}

impl DataViewEvent {
    pub const fn new(kind: DataViewEventKind, item_index: usize, column: usize) -> Self {
        Self {
            kind,
            item_index,
            column,
        }
    }
}
