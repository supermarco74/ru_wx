//! XamlTab — TabView container with multiple pages.

use super::super::element_bridge::{next_id, TabDesc, WidgetDesc, WidgetId};
use super::super::window::XamlFrame;

/// Tab container (TabView).
pub struct XamlTab {
    id: WidgetId,
    frame: XamlFrame,
}

impl XamlTab {
    pub fn new(frame: &XamlFrame) -> Self {
        let id = next_id();
        frame.desc.borrow_mut().children.push(WidgetDesc::TabView {
            id,
            tabs: Vec::new(),
            selected: 0,
        });
        Self {
            id,
            frame: frame.clone(),
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// Add a tab page.
    pub fn add_page(&self, header: &str, content: WidgetDesc) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::TabView { id, tabs, .. } = child {
                if *id == self.id {
                    tabs.push(TabDesc {
                        header: header.to_string(),
                        content,
                        closable: false,
                    });
                    break;
                }
            }
        }
    }

    /// Add a closable tab page.
    pub fn add_page_closable(&self, header: &str, content: WidgetDesc) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::TabView { id, tabs, .. } = child {
                if *id == self.id {
                    tabs.push(TabDesc {
                        header: header.to_string(),
                        content,
                        closable: true,
                    });
                    break;
                }
            }
        }
    }

    /// Set the selected tab index.
    pub fn set_selected(&self, index: i32) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::TabView { id, selected, .. } = child {
                if *id == self.id {
                    *selected = index;
                    break;
                }
            }
        }
    }
}
