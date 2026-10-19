//! XamlScrolledWindow — scrollable container.

use super::super::element_bridge::{next_id, WidgetDesc, WidgetId};
use super::super::window::XamlFrame;

/// Scrollable container (ScrollViewer).
pub struct XamlScrolledWindow {
    id: WidgetId,
    frame: XamlFrame,
}

impl XamlScrolledWindow {
    pub fn new(frame: &XamlFrame, child: WidgetDesc) -> Self {
        let id = next_id();
        frame
            .desc
            .borrow_mut()
            .children
            .push(WidgetDesc::ScrollViewer {
                id,
                child: Box::new(child),
                horizontal_scroll: false,
                vertical_scroll: true,
            });
        Self {
            id,
            frame: frame.clone(),
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// Enable horizontal scrolling.
    pub fn set_horizontal_scroll(&self, enabled: bool) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::ScrollViewer {
                id,
                horizontal_scroll,
                ..
            } = child
            {
                if *id == self.id {
                    *horizontal_scroll = enabled;
                    break;
                }
            }
        }
    }

    /// Enable vertical scrolling.
    pub fn set_vertical_scroll(&self, enabled: bool) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::ScrollViewer {
                id,
                vertical_scroll,
                ..
            } = child
            {
                if *id == self.id {
                    *vertical_scroll = enabled;
                    break;
                }
            }
        }
    }
}
