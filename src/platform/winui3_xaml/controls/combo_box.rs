//! XamlComboBox — dropdown / combo box widget.

use super::super::element_bridge::{next_id, WidgetDesc, WidgetId};
use super::super::window::XamlFrame;

/// Dropdown combo box.
pub struct XamlComboBox {
    id: WidgetId,
    frame: XamlFrame,
}

impl XamlComboBox {
    pub fn new(frame: &XamlFrame, items: &[&str]) -> Self {
        let id = next_id();
        frame.desc.borrow_mut().children.push(WidgetDesc::ComboBox {
            id,
            items: items.iter().map(|s| s.to_string()).collect(),
            selected: None,
            editable: false,
            width: 0.0,
        });
        Self {
            id,
            frame: frame.clone(),
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// Imposta la larghezza fissa (px logici). `0.0` = automatica.
    pub fn set_width(&self, width: f64) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::ComboBox { id, width: w, .. } = child {
                if *id == self.id {
                    *w = width;
                    break;
                }
            }
        }
    }

    pub fn set_selected(&self, index: usize) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::ComboBox { id, selected, .. } = child {
                if *id == self.id {
                    *selected = Some(index);
                    break;
                }
            }
        }
    }

    pub fn on_selection_changed<F>(&self, f: F)
    where
        F: FnMut(usize) + Send + 'static,
    {
        let desc = self.frame.desc.borrow();
        if let Ok(mut reg) = desc.registry.lock() {
            reg.selection_changed_handlers.insert(self.id, Box::new(f));
        };
    }
}
