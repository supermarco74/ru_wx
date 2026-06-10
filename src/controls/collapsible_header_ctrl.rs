//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Collapsible section header (`wxCollapsibleHeaderCtrl`).

use std::cell::RefCell;
use std::rc::Rc;

use crate::controls::button::Button;
use crate::core::widget::{WidgetRef, Window};
use crate::window::frame::Frame;

/// Clickable header that expands/collapses a section (`wxCollapsibleHeaderCtrl`).
#[derive(Clone)]
pub struct CollapsibleHeaderCtrl {
    header: Button,
    expanded: Rc<RefCell<bool>>,
    label: String,
}

impl CollapsibleHeaderCtrl {
    pub fn new<W: Window>(parent: &W, label: &str) -> Self {
        Self {
            header: Button::new(parent, &format!("▼ {label}")),
            expanded: Rc::new(RefCell::new(true)),
            label: label.to_string(),
        }
    }

    pub fn is_expanded(&self) -> bool {
        *self.expanded.borrow()
    }

    pub fn set_expanded(&self, expanded: bool) {
        *self.expanded.borrow_mut() = expanded;
        let prefix = if expanded { "▼" } else { "▶" };
        self.header.set_label(&format!("{prefix} {}", self.label));
    }

    pub fn bind_toggle<F: FnMut(bool) + 'static>(&self, frame: &Frame, mut on_toggle: F) {
        let this = self.clone();
        let header = this.header.clone();
        header.on_click(frame, move || {
            let next = !*this.expanded.borrow();
            this.set_expanded(next);
            on_toggle(next);
        });
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.header.as_widget_ref()
    }
}
