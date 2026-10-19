//! XamlListBox — flat list widget.

use super::super::element_bridge::{next_id, WidgetDesc, WidgetId};
use super::super::window::XamlFrame;

/// List box (flat list of strings).
pub struct XamlListBox {
    id: WidgetId,
    frame: XamlFrame,
}

impl XamlListBox {
    pub fn new(frame: &XamlFrame) -> Self {
        let id = next_id();
        frame.desc.borrow_mut().children.push(WidgetDesc::ListBox {
            id,
            items: Vec::new(),
            selected: -1,
        });
        Self {
            id,
            frame: frame.clone(),
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// Add an item.
    pub fn append(&self, text: &str) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::ListBox { id, items, .. } = child {
                if *id == self.id {
                    items.push(text.to_string());
                    break;
                }
            }
        }
    }

    /// Set the selected index.
    pub fn set_selected(&self, index: i32) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::ListBox { id, selected, .. } = child {
                if *id == self.id {
                    *selected = index;
                    break;
                }
            }
        }
    }

    /// Clear all items.
    pub fn clear(&self) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::ListBox { id, items, .. } = child {
                if *id == self.id {
                    items.clear();
                    break;
                }
            }
        }
    }
}
