//! XamlStatusBar — bottom status bar with text fields.

use super::super::element_bridge::{next_id, StatusFieldDesc, WidgetDesc, WidgetId};
use super::super::window::XamlFrame;

/// Status bar (bottom bar with text fields).
pub struct XamlStatusBar {
    id: WidgetId,
    frame: XamlFrame,
}

impl XamlStatusBar {
    pub fn new(frame: &XamlFrame, field_count: usize) -> Self {
        let id = next_id();
        let fields = (0..field_count)
            .map(|_| StatusFieldDesc {
                text: String::new(),
                width: None,
            })
            .collect();
        frame
            .desc
            .borrow_mut()
            .children
            .push(WidgetDesc::StatusBar { id, fields });
        Self {
            id,
            frame: frame.clone(),
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// Set the text of a field.
    pub fn set_status_text(&self, text: &str, field: usize) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::StatusBar { id, fields } = child {
                if *id == self.id {
                    if let Some(f) = fields.get_mut(field) {
                        f.text = text.to_string();
                    }
                    break;
                }
            }
        }
    }

    /// Set the width of a field.
    pub fn set_field_width(&self, field: usize, width: f64) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::StatusBar { id, fields } = child {
                if *id == self.id {
                    if let Some(f) = fields.get_mut(field) {
                        f.width = Some(width);
                    }
                    break;
                }
            }
        }
    }
}
