//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! List header control (`wxHeaderCtrl`).

use std::cell::RefCell;

use crate::core::geometry::Rect;
use crate::core::header_events::{HeaderButtonClickEvent, HeaderColumnEvent};

/// Replaceable event callback slot used by [`HeaderCtrl`].
type HeaderHandler<E> = RefCell<Option<Box<dyn FnMut(&E)>>>;

/// Column header bar (`wxHeaderCtrl`).
pub struct HeaderCtrl {
    columns: Vec<HeaderColumn>,
    rect: Rect,
    sort_column: Option<usize>,
    on_button_click: HeaderHandler<HeaderButtonClickEvent>,
    on_column_event: HeaderHandler<HeaderColumnEvent>,
}

/// One sortable column (`wxHeaderColumn`).
#[derive(Debug, Clone)]
pub struct HeaderColumn {
    pub title: String,
    pub width: u32,
    pub visible: bool,
}

impl HeaderCtrl {
    pub fn new(rect: Rect) -> Self {
        Self {
            columns: Vec::new(),
            rect,
            sort_column: None,
            on_button_click: RefCell::new(None),
            on_column_event: RefCell::new(None),
        }
    }

    /// Register a callback for column resize/reorder events.
    pub fn on_column_event<F: FnMut(&HeaderColumnEvent) + 'static>(&self, f: F) {
        *self.on_column_event.borrow_mut() = Some(Box::new(f));
    }

    /// Simulate a column resize (for tests / stubs).
    pub fn resize_column(&self, column: usize, width: u32) {
        if let Some(ref mut cb) = *self.on_column_event.borrow_mut() {
            cb(&HeaderColumnEvent::new(column, width));
        }
    }

    /// Register a callback for header button clicks.
    pub fn on_button_click<F: FnMut(&HeaderButtonClickEvent) + 'static>(&self, f: F) {
        *self.on_button_click.borrow_mut() = Some(Box::new(f));
    }

    /// Simulate a column header button click (for tests / stubs).
    pub fn click_column(&self, column: usize) {
        if let Some(ref mut cb) = *self.on_button_click.borrow_mut() {
            cb(&HeaderButtonClickEvent::new(column));
        }
    }

    pub fn append_column(&mut self, title: &str, width: u32) -> usize {
        self.columns.push(HeaderColumn {
            title: title.to_string(),
            width,
            visible: true,
        });
        self.columns.len() - 1
    }

    pub fn set_sort_column(&mut self, index: Option<usize>) {
        self.sort_column = index;
    }

    pub fn sort_column(&self) -> Option<usize> {
        self.sort_column
    }

    pub fn columns(&self) -> &[HeaderColumn] {
        &self.columns
    }

    pub fn rect(&self) -> Rect {
        self.rect
    }
}
