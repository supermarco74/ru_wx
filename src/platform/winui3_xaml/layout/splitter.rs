//! XamlSplitter — SplitView with collapsible pane and content area.

use super::super::element_bridge::{next_id, WidgetDesc, WidgetId};
use super::super::window::XamlFrame;

/// Splitter container (SplitView).
///
/// The pane is on the left, content on the right.
/// The pane can be collapsed/expanded.
pub struct XamlSplitter {
    id: WidgetId,
    frame: XamlFrame,
}

impl XamlSplitter {
    pub fn new(frame: &XamlFrame, pane: WidgetDesc, content: WidgetDesc) -> Self {
        let id = next_id();
        frame
            .desc
            .borrow_mut()
            .children
            .push(WidgetDesc::SplitView {
                id,
                pane: Box::new(pane),
                content: Box::new(content),
                pane_open: true,
                pane_length: 250.0,
            });
        Self {
            id,
            frame: frame.clone(),
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// Set the pane width.
    pub fn set_pane_length(&self, length: f64) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::SplitView {
                id, pane_length, ..
            } = child
            {
                if *id == self.id {
                    *pane_length = length;
                    break;
                }
            }
        }
    }

    /// Open or close the pane.
    pub fn set_pane_open(&self, open: bool) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::SplitView { id, pane_open, .. } = child {
                if *id == self.id {
                    *pane_open = open;
                    break;
                }
            }
        }
    }
}
