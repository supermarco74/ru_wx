//! XamlCheckListBox — lista piatta con una check-box per riga.
//!
//! Omologa XAML della `CheckListBox` di AppKit (`NSTableView` con celle
//! checkbox). Il mapping in `app.rs` la rende come `vstack` di `CheckBox`.

use super::super::element_bridge::{next_id, WidgetDesc, WidgetId};
use super::super::window::XamlFrame;

/// Check list box (flat list of items with per-item check state).
pub struct XamlCheckListBox {
    id: WidgetId,
    frame: XamlFrame,
}

impl XamlCheckListBox {
    pub fn new(frame: &XamlFrame) -> Self {
        let id = next_id();
        frame
            .desc
            .borrow_mut()
            .children
            .push(WidgetDesc::CheckListBox {
                id,
                items: Vec::new(),
                checked: Vec::new(),
            });
        Self {
            id,
            frame: frame.clone(),
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// Add an item (unchecked by default).
    pub fn append(&self, text: &str) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::CheckListBox { id, items, checked } = child {
                if *id == self.id {
                    items.push(text.to_string());
                    checked.push(false);
                    break;
                }
            }
        }
    }

    /// Set the checked state of an item.
    pub fn set_checked(&self, index: usize, checked: bool) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::CheckListBox {
                id, checked: flags, ..
            } = child
            {
                if *id == self.id {
                    if let Some(flag) = flags.get_mut(index) {
                        *flag = checked;
                    }
                    break;
                }
            }
        }
    }

    /// Read the checked state of an item.
    pub fn is_checked(&self, index: usize) -> bool {
        let desc = self.frame.desc.borrow();
        for child in &desc.children {
            if let WidgetDesc::CheckListBox { id, checked, .. } = child {
                if *id == self.id {
                    return checked.get(index).copied().unwrap_or(false);
                }
            }
        }
        false
    }

    /// Clear all items and their checked states.
    pub fn clear(&self) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::CheckListBox {
                id, items, checked, ..
            } = child
            {
                if *id == self.id {
                    items.clear();
                    checked.clear();
                    break;
                }
            }
        }
    }
}
