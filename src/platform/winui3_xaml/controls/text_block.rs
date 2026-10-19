//! XamlTextBlock — static text label widget.

use super::super::element_bridge::{next_id, WidgetDesc, WidgetId};
use super::super::window::XamlFrame;

/// Static text label.
pub struct XamlTextBlock {
    id: WidgetId,
    frame: XamlFrame,
}

impl XamlTextBlock {
    pub fn new(frame: &XamlFrame, text: &str) -> Self {
        let id = next_id();
        frame
            .desc
            .borrow_mut()
            .children
            .push(WidgetDesc::TextBlock {
                id,
                text: text.to_string(),
                font_size: 0.0,
                bold: false,
            });
        Self {
            id,
            frame: frame.clone(),
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    pub fn set_label(&self, text: &str) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::TextBlock { id, text: t, .. } = child {
                if *id == self.id {
                    *t = text.to_string();
                    break;
                }
            }
        }
    }

    pub fn set_font_size(&self, size: f64) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::TextBlock { id, font_size, .. } = child {
                if *id == self.id {
                    *font_size = size;
                    break;
                }
            }
        }
    }

    pub fn set_bold(&self, bold: bool) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::TextBlock { id, bold: b, .. } = child {
                if *id == self.id {
                    *b = bold;
                    break;
                }
            }
        }
    }
}
