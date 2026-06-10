//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Top-of-window banner (`wxBannerWindow`).

use crate::controls::button::Button;
use crate::controls::static_text::StaticText;
use crate::core::geometry::Colour;
use crate::core::widget::{WidgetRef, Window};
use crate::window::frame::Frame;

#[derive(Clone)]
pub struct BannerWindow {
    title: StaticText,
    close: Button,
    colour: Colour,
}

impl BannerWindow {
    pub fn new<W: Window>(parent: &W, message: &str) -> Self {
        Self {
            title: StaticText::new(parent, message),
            close: Button::new(parent, "×"),
            colour: Colour::new(0, 120, 215, 255),
        }
    }

    pub fn set_message(&self, text: &str) {
        self.title.set_label(text);
    }

    pub fn set_colour(&mut self, colour: Colour) {
        self.colour = colour;
    }

    pub fn colour(&self) -> Colour {
        self.colour
    }

    pub fn bind_close(&self, frame: &Frame) {
        let title = self.title.clone();
        let bid = self.close.id();
        frame.register_command_handler(bid, Box::new(move || title.set_label("")));
    }

    pub fn message_widget(&self) -> WidgetRef {
        self.title.as_widget_ref()
    }

    pub fn close_widget(&self) -> WidgetRef {
        self.close.as_widget_ref()
    }
}
