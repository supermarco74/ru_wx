//! XamlToolBar — toolbar with buttons.

use super::super::element_bridge::{next_id, ToolBarItemDesc, WidgetDesc, WidgetId};
use super::super::window::XamlFrame;

/// Toolbar (command bar with buttons and separators).
pub struct XamlToolBar {
    id: WidgetId,
    frame: XamlFrame,
}

impl XamlToolBar {
    pub fn new(frame: &XamlFrame) -> Self {
        let id = next_id();
        frame.desc.borrow_mut().children.push(WidgetDesc::ToolBar {
            id,
            items: Vec::new(),
        });
        Self {
            id,
            frame: frame.clone(),
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// Add a button.
    pub fn add_button(&self, label: &str) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::ToolBar { id, items } = child {
                if *id == self.id {
                    items.push(ToolBarItemDesc::Button {
                        label: label.to_string(),
                    });
                    break;
                }
            }
        }
    }

    /// Add a toggle button.
    pub fn add_toggle(&self, label: &str) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::ToolBar { id, items } = child {
                if *id == self.id {
                    items.push(ToolBarItemDesc::Toggle {
                        label: label.to_string(),
                    });
                    break;
                }
            }
        }
    }

    /// Add a separator.
    pub fn add_separator(&self) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::ToolBar { id, items } = child {
                if *id == self.id {
                    items.push(ToolBarItemDesc::Separator);
                    break;
                }
            }
        }
    }
}
