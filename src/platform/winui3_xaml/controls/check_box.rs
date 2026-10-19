//! XamlCheckBox — checkbox widget.

use super::super::element_bridge::{next_id, WidgetDesc, WidgetId};
use super::super::window::XamlFrame;

/// Checkbox with label.
pub struct XamlCheckBox {
    id: WidgetId,
    frame: XamlFrame,
}

impl XamlCheckBox {
    pub fn new(frame: &XamlFrame, label: &str, checked: bool) -> Self {
        let id = next_id();
        frame.desc.borrow_mut().children.push(WidgetDesc::CheckBox {
            id,
            label: label.to_string(),
            checked,
        });
        Self {
            id,
            frame: frame.clone(),
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    pub fn set_checked(&self, checked: bool) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::CheckBox { id, checked: c, .. } = child {
                if *id == self.id {
                    *c = checked;
                    break;
                }
            }
        }
    }

    pub fn on_toggled<F>(&self, f: F)
    where
        F: FnMut(bool) + Send + 'static,
    {
        let desc = self.frame.desc.borrow();
        if let Ok(mut reg) = desc.registry.lock() {
            reg.toggled_handlers.insert(self.id, Box::new(f));
        };
    }
}
