//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Virtual list box (`wxVListBox`).

use std::cell::RefCell;
use std::rc::Rc;

use crate::controls::list_ctrl::{ListCtrl, ListCtrlStyle};
use crate::core::widget::WidgetRef;
use crate::window::frame::Frame;

/// Owner-data list with on-demand line text (`wxVListBox`).
#[derive(Clone)]
pub struct VListBox {
    list: ListCtrl,
    lines: Rc<RefCell<Vec<String>>>,
}

impl VListBox {
    /// Create a virtual list bound to `frame` for `LVN_GETDISPINFOW` dispatch.
    pub fn new(frame: &Frame) -> Self {
        let list = ListCtrl::new(frame, ListCtrlStyle::Report);
        list.insert_column(0, "", 200);
        let lines = Rc::new(RefCell::new(Vec::<String>::new()));
        let lines_cb = lines.clone();
        list.on_get_disp_info(frame, move |item| {
            if item.is_text_requested() && item.sub_item() == 0 {
                let idx = item.index();
                let text = lines_cb
                    .borrow()
                    .get(idx)
                    .cloned()
                    .unwrap_or_default();
                let _ = item.set_text(&text);
            }
        });
        Self { list, lines }
    }

    pub fn set_line_count(&self, count: usize) {
        let mut lines = self.lines.borrow_mut();
        lines.resize(count, String::new());
        self.list.set_item_count(count as u32);
    }

    pub fn set_line(&self, index: usize, text: &str) {
        let changed = {
            let mut lines = self.lines.borrow_mut();
            if index < lines.len() {
                lines[index] = text.to_string();
                true
            } else {
                false
            }
        };
        // In owner-data mode the ListView caches the painted text;
        // invalidate the row so it re-queries LVN_GETDISPINFO.
        if changed {
            self.list.redraw_items(index, index);
        }
    }

    pub fn line_count(&self) -> usize {
        self.lines.borrow().len()
    }

    pub fn line(&self, index: usize) -> Option<String> {
        self.lines.borrow().get(index).cloned()
    }

    pub fn selection(&self) -> Option<usize> {
        let sel = self.list.get_selected_items();
        sel.first().copied()
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.list.as_widget_ref()
    }
}
