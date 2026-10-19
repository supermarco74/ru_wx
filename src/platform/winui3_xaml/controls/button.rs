//! XamlButton — push button widget.
//!
//! Mirrors `ru_wx::Button` API: `new(parent, label)`, `on_click`, `set_label`,
//! `set_enabled`.

use super::super::element_bridge::{next_id, WidgetDesc, WidgetId};
use super::super::window::XamlFrame;

/// Push button widget.
///
/// Created with a parent frame and a label. The button appends itself
/// to the frame's widget tree and registers event handlers in the
/// frame's event registry.
pub struct XamlButton {
    id: WidgetId,
    frame: XamlFrame,
}

impl XamlButton {
    /// Create a new button with the given label.
    pub fn new(frame: &XamlFrame, label: &str) -> Self {
        let id = next_id();
        frame.desc.borrow_mut().children.push(WidgetDesc::Button {
            id,
            label: label.to_string(),
            enabled: true,
            width: 0.0,
        });
        Self {
            id,
            frame: frame.clone(),
        }
    }

    /// Get the widget ID.
    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// Register a click handler.
    pub fn on_click<F>(&self, f: F)
    where
        F: FnMut() + Send + 'static,
    {
        let desc = self.frame.desc.borrow();
        if let Ok(mut reg) = desc.registry.lock() {
            reg.click_handlers.insert(self.id, Box::new(f));
        };
    }

    /// Set the button label at runtime.
    pub fn set_label(&self, label: &str) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::Button { id, label: l, .. } = child {
                if *id == self.id {
                    *l = label.to_string();
                    break;
                }
            }
        }
    }

    /// Enable or disable the button.
    pub fn set_enabled(&self, enabled: bool) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::Button { id, enabled: e, .. } = child {
                if *id == self.id {
                    *e = enabled;
                    break;
                }
            }
        }
    }

    /// Imposta la larghezza fissa (px logici). `0.0` = automatica.
    pub fn set_width(&self, width: f64) {
        let mut desc = self.frame.desc.borrow_mut();
        for child in &mut desc.children {
            if let WidgetDesc::Button { id, width: w, .. } = child {
                if *id == self.id {
                    *w = width;
                    break;
                }
            }
        }
    }
}
