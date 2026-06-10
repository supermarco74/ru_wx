//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Information banner (`wxInfoBar`).

use crate::controls::button::Button;
use crate::controls::static_text::StaticText;
use crate::core::geometry::Colour;
use crate::core::widget::{WidgetRef, Window};
use crate::window::frame::Frame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoBarMessageType {
    Info,
    Warning,
    Error,
}

#[derive(Clone)]
pub struct InfoBar {
    label: StaticText,
    dismiss: Button,
    kind: InfoBarMessageType,
}

impl InfoBar {
    pub fn new<W: Window>(parent: &W) -> Self {
        Self {
            label: StaticText::new(parent, ""),
            dismiss: Button::new(parent, "×"),
            kind: InfoBarMessageType::Info,
        }
    }

    pub fn show_message(&mut self, text: &str, kind: InfoBarMessageType) {
        self.kind = kind;
        self.label.set_label(text);
    }

    pub fn bind_dismiss(&self, frame: &Frame) {
        let label = self.label.clone();
        let bid = self.dismiss.id();
        frame.register_command_handler(bid, Box::new(move || label.set_label("")));
    }

    /// Dismiss with [`InfoBarEvent`] payload (`wxInfoBarEvent`).
    pub fn on_info_bar_event<F: FnMut(&crate::chrome::info_bar_event::InfoBarEvent) + 'static>(
        &self,
        frame: &Frame,
        mut f: F,
    ) {
        let kind = self.kind;
        let bid = self.dismiss.id();
        frame.register_command_handler(bid, Box::new(move || {
            f(&crate::chrome::info_bar_event::InfoBarEvent::new(
                crate::chrome::info_bar_event::InfoBarEventKind::Dismissed,
                kind,
                "",
            ));
        }));
    }

    pub fn background_colour(&self) -> Colour {
        match self.kind {
            InfoBarMessageType::Info => Colour::new(220, 235, 255, 255),
            InfoBarMessageType::Warning => Colour::new(255, 245, 200, 255),
            InfoBarMessageType::Error => Colour::new(255, 220, 220, 255),
        }
    }

    pub fn label_widget(&self) -> WidgetRef {
        self.label.as_widget_ref()
    }

    pub fn dismiss_widget(&self) -> WidgetRef {
        self.dismiss.as_widget_ref()
    }
}
