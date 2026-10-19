//! XamlColorDialog — color picker dialog.

use super::super::element_bridge::{next_id, WidgetDesc, WidgetId};
use super::super::window::XamlFrame;

/// Color picker dialog.
pub struct XamlColorDialog {
    id: WidgetId,
    frame: XamlFrame,
}

impl XamlColorDialog {
    pub fn new(frame: &XamlFrame, r: u8, g: u8, b: u8) -> Self {
        let id = next_id();
        frame
            .desc
            .borrow_mut()
            .children
            .push(WidgetDesc::ColorPicker {
                id,
                color: (255, r, g, b),
                is_open: false,
            });
        Self {
            id,
            frame: frame.clone(),
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// Show the picker (adds it to the layout).
    pub fn show(&self) {
        self.set_open(true);
    }

    /// Hide the picker (frees its layout space).
    pub fn close(&self) {
        self.set_open(false);
    }

    fn set_open(&self, open: bool) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::ColorPicker { id, is_open, .. } = child {
                if *id == self.id {
                    *is_open = open;
                    break;
                }
            }
        }
    }

    /// Set the current color.
    pub fn set_color(&self, r: u8, g: u8, b: u8) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::ColorPicker { id, color, .. } = child {
                if *id == self.id {
                    *color = (255, r, g, b);
                    break;
                }
            }
        }
    }
}
