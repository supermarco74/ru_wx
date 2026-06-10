//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Data-view controls (`wxDataViewCtrl` family) — model/view skeleton.

use std::cell::RefCell;
use std::rc::Rc;

use crate::controls::list_ctrl::{ListCtrl, ListCtrlStyle};
use crate::controls::tree_ctrl::{TreeCtrl, TreeItem};
use crate::core::widget::WidgetRef;
use crate::window::frame::Frame;

/// Column descriptor (`wxDataViewColumn`).
#[derive(Debug, Clone)]
pub struct DataViewColumn {
    pub title: String,
    pub width: i32,
    pub align_right: bool,
}

/// Cell renderer trait (`wxDataViewRenderer`).
pub trait DataViewRenderer: Send {
    fn render_text(&self, value: &str) -> String;
}

#[derive(Default)]
pub struct TextRenderer;

impl DataViewRenderer for TextRenderer {
    fn render_text(&self, value: &str) -> String {
        value.to_string()
    }
}

/// Generic data-view host (`wxDataViewCtrl`).
#[derive(Clone)]
pub struct DataViewCtrl {
    list: ListCtrl,
    columns: Rc<RefCell<Vec<DataViewColumn>>>,
}

impl DataViewCtrl {
    pub fn new<W: crate::core::widget::Window>(parent: &W) -> Self {
        Self {
            list: ListCtrl::new(parent, ListCtrlStyle::Report),
            columns: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn append_column(&self, title: &str, width: i32) {
        self.columns.borrow_mut().push(DataViewColumn {
            title: title.to_string(),
            width,
            align_right: false,
        });
        let idx = self.columns.borrow().len() as u32 - 1;
        self.list.insert_column(idx, title, width);
    }

    pub fn append_item(&self, values: &[&str]) {
        let idx = self.list.insert_item(self.list.get_item_count(), values.first().copied().unwrap_or(""));
        for (col, v) in values.iter().enumerate().skip(1) {
            self.list.set_item_text(idx, col, v);
        }
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.list.as_widget_ref()
    }
}

/// List-shaped data view (`wxDataViewListCtrl`).
pub type DataViewListCtrl = DataViewCtrl;

/// Tree-shaped data view (`wxDataViewTreeCtrl`).
#[derive(Clone)]
pub struct DataViewTreeCtrl {
    tree: TreeCtrl,
}

impl DataViewTreeCtrl {
    pub fn new(frame: &Frame) -> Self {
        Self {
            tree: TreeCtrl::new(frame),
        }
    }

    pub fn append_root(&self, label: &str) -> TreeItem {
        self.tree.add_root(label)
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.tree.as_widget_ref()
    }
}
