//! XamlTextBox — text input widget.

use super::super::element_bridge::{next_id, WidgetDesc, WidgetId};
use super::super::window::XamlFrame;

/// Single-line or multi-line text input.
pub struct XamlTextBox {
    id: WidgetId,
    frame: XamlFrame,
}

impl XamlTextBox {
    /// Create a single-line text box.
    pub fn new(frame: &XamlFrame, text: &str) -> Self {
        let id = next_id();
        frame.desc.borrow_mut().children.push(WidgetDesc::TextBox {
            id,
            text: text.to_string(),
            placeholder: String::new(),
            read_only: false,
        });
        Self {
            id,
            frame: frame.clone(),
        }
    }

    /// Create a multi-line text area.
    pub fn multiline(frame: &XamlFrame, text: &str) -> Self {
        let id = next_id();
        frame.desc.borrow_mut().children.push(WidgetDesc::TextArea {
            id,
            text: text.to_string(),
            read_only: false,
        });
        Self {
            id,
            frame: frame.clone(),
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    pub fn set_value(&self, text: &str) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            match child {
                WidgetDesc::TextBox { id, text: t, .. } if *id == self.id => {
                    *t = text.to_string();
                    break;
                }
                WidgetDesc::TextArea { id, text: t, .. } if *id == self.id => {
                    *t = text.to_string();
                    break;
                }
                _ => {}
            }
        }
    }

    pub fn set_placeholder(&self, text: &str) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::TextBox {
                id, placeholder, ..
            } = child
            {
                if *id == self.id {
                    *placeholder = text.to_string();
                    break;
                }
            }
        }
    }

    pub fn on_text_changed<F>(&self, f: F)
    where
        F: FnMut(String) + Send + 'static,
    {
        let desc = self.frame.desc.borrow();
        if let Ok(mut reg) = desc.registry.lock() {
            reg.text_changed_handlers.insert(self.id, Box::new(f));
        };
    }
}
