//! XamlMenuBar — top-level menu bar.

use super::super::element_bridge::{next_id, MenuDesc, MenuItemDesc, WidgetDesc, WidgetId};
use super::super::window::XamlFrame;

/// Menu bar (top-level menus with dropdowns).
pub struct XamlMenuBar {
    id: WidgetId,
    frame: XamlFrame,
}

impl XamlMenuBar {
    pub fn new(frame: &XamlFrame) -> Self {
        let id = next_id();
        frame.desc.borrow_mut().children.push(WidgetDesc::MenuBar {
            id,
            menus: Vec::new(),
        });
        Self {
            id,
            frame: frame.clone(),
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// Add a top-level menu.
    pub fn add_menu(&self, menu: XamlMenu) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::MenuBar { id, menus } = child {
                if *id == self.id {
                    menus.push(menu.desc);
                    break;
                }
            }
        }
    }
}

/// A single top-level menu (e.g. "File", "Edit").
pub struct XamlMenu {
    desc: MenuDesc,
}

impl XamlMenu {
    pub fn new(title: &str) -> Self {
        Self {
            desc: MenuDesc {
                title: title.to_string(),
                items: Vec::new(),
            },
        }
    }

    /// Add a clickable item.
    pub fn add_item(&mut self, text: &str) {
        self.desc.items.push(MenuItemDesc::Item {
            text: text.to_string(),
            enabled: true,
        });
    }

    /// Add a disabled item.
    pub fn add_item_disabled(&mut self, text: &str) {
        self.desc.items.push(MenuItemDesc::Item {
            text: text.to_string(),
            enabled: false,
        });
    }

    /// Add a separator.
    pub fn add_separator(&mut self) {
        self.desc.items.push(MenuItemDesc::Separator);
    }

    /// Add a submenu.
    pub fn add_submenu(&mut self, text: &str, submenu: XamlMenu) {
        self.desc.items.push(MenuItemDesc::SubMenu {
            text: text.to_string(),
            children: submenu.desc.items,
        });
    }
}
