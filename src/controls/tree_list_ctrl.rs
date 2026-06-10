//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Tree + list hybrid (`wxTreeListCtrl`) — columnar tree skeleton.

use std::cell::RefCell;
use std::rc::Rc;

use crate::controls::tree_ctrl::{TreeCtrl, TreeItem};
use crate::core::widget::WidgetRef;
use crate::window::frame::Frame;

/// One column definition in a [`TreeListCtrl`].
#[derive(Debug, Clone)]
pub struct TreeListColumn {
    pub title: String,
    pub width: u32,
}

/// Tree-list row values (one string per column).
#[derive(Debug, Clone, Default)]
pub struct TreeListRow {
    pub cells: Vec<String>,
}

#[derive(Clone)]
pub struct TreeListCtrl {
    tree: TreeCtrl,
    columns: Rc<RefCell<Vec<TreeListColumn>>>,
    rows: Rc<RefCell<Vec<TreeListRow>>>,
}

impl TreeListCtrl {
    pub fn new(frame: &Frame) -> Self {
        Self {
            tree: TreeCtrl::new(frame),
            columns: Rc::new(RefCell::new(Vec::new())),
            rows: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn append_column(&self, title: &str, width: u32) {
        self.columns
            .borrow_mut()
            .push(TreeListColumn {
                title: title.to_string(),
                width,
            });
    }

    pub fn append_row(&self, parent: TreeItem, cells: &[&str]) {
        let label = cells.first().copied().unwrap_or("");
        let item = self.tree.append_item(parent, label);
        self.rows.borrow_mut().push(TreeListRow {
            cells: cells.iter().map(|s| (*s).to_string()).collect(),
        });
        let _ = item;
    }

    pub fn column_count(&self) -> usize {
        self.columns.borrow().len()
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.tree.as_widget_ref()
    }
}
