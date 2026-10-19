//! XamlMessageDialog — modal message dialog.

use super::super::element_bridge::{next_id, WidgetDesc, WidgetId};
use super::super::window::XamlFrame;

/// Modal message dialog (ContentDialog).
pub struct XamlMessageDialog {
    id: WidgetId,
    frame: XamlFrame,
}

impl XamlMessageDialog {
    /// Create a message dialog with title, content, and button texts.
    pub fn new(
        frame: &XamlFrame,
        title: &str,
        content: &str,
        primary: &str,
        secondary: &str,
        close: &str,
    ) -> Self {
        let id = next_id();
        frame
            .desc
            .borrow_mut()
            .children
            .push(WidgetDesc::ContentDialog {
                id,
                title: title.to_string(),
                content: content.to_string(),
                primary_text: primary.to_string(),
                secondary_text: secondary.to_string(),
                close_text: close.to_string(),
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

    /// Open the dialog.
    pub fn show(&self) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::ContentDialog { id, is_open, .. } = child {
                if *id == self.id {
                    *is_open = true;
                    break;
                }
            }
        }
    }

    /// Close the dialog.
    pub fn close(&self) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::ContentDialog { id, is_open, .. } = child {
                if *id == self.id {
                    *is_open = false;
                    break;
                }
            }
        }
    }
}
