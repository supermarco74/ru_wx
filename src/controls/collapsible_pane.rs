//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Expandable section (`wxCollapsiblePane`).

use std::cell::RefCell;
use std::rc::Rc;

use crate::controls::button::Button;
use crate::core::widget::{WidgetRef, Window};
use crate::window::frame::Frame;

#[derive(Clone)]
pub struct CollapsiblePane {
    toggle: Button,
    body: WidgetRef,
    expanded: Rc<RefCell<bool>>,
    label_expanded: String,
    label_collapsed: String,
}

impl CollapsiblePane {
    pub fn new<W: Window>(parent: &W, label: &str, body: WidgetRef) -> Self {
        let expanded = Rc::new(RefCell::new(true));
        let toggle = Button::new(parent, &format!("▼ {label}"));
        Self {
            toggle,
            body,
            expanded,
            label_expanded: format!("▼ {label}"),
            label_collapsed: format!("▶ {label}"),
        }
    }

    pub fn set_expanded(&self, expanded: bool) {
        *self.expanded.borrow_mut() = expanded;
        self.toggle
            .set_label(if expanded { &self.label_expanded } else { &self.label_collapsed });
        self.body.borrow_mut().set_visible(expanded);
    }

    pub fn is_expanded(&self) -> bool {
        *self.expanded.borrow()
    }

    pub fn bind_toggle(&self, frame: &Frame) {
        self.bind_toggle_with_event(frame, |_| {});
    }

    /// Toggle with [`CollapsiblePaneEvent`] callback.
    pub fn bind_toggle_with_event<F: FnMut(&crate::CollapsiblePaneEvent) + 'static>(
        &self,
        frame: &Frame,
        mut on_event: F,
    ) {
        let this = self.clone();
        self.toggle.on_click(frame, move || {
            let next = !*this.expanded.borrow();
            this.set_expanded(next);
            on_event(&crate::CollapsiblePaneEvent::new(next));
        });
    }

    pub fn toggle_widget(&self) -> WidgetRef {
        self.toggle.as_widget_ref()
    }

    pub fn body_widget(&self) -> WidgetRef {
        self.body.clone()
    }
}
