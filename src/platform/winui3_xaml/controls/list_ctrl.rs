//! XamlListCtrl — table with columns.

use super::super::element_bridge::{next_id, WidgetDesc, WidgetId};
use super::super::window::XamlFrame;

/// List control (table with columns and rows).
pub struct XamlListCtrl {
    id: WidgetId,
    frame: XamlFrame,
}

impl XamlListCtrl {
    pub fn new(frame: &XamlFrame) -> Self {
        let id = next_id();
        frame.desc.borrow_mut().children.push(WidgetDesc::ListCtrl {
            id,
            columns: Vec::new(),
            rows: Vec::new(),
        });
        Self {
            id,
            frame: frame.clone(),
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// Add a column.
    pub fn add_column(&self, title: &str) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::ListCtrl { id, columns, .. } = child {
                if *id == self.id {
                    columns.push(title.to_string());
                    break;
                }
            }
        }
    }

    /// Add a row of values.
    pub fn add_row(&self, values: &[&str]) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::ListCtrl { id, rows, .. } = child {
                if *id == self.id {
                    rows.push(values.iter().map(|s| s.to_string()).collect());
                    break;
                }
            }
        }
    }
}
