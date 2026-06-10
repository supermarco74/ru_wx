//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Toast-style notification (`wxNotificationMessage`).

use crate::chrome::icon_tray::{BalloonIcon, IconTray};
use crate::window::frame::Frame;

/// Short-lived notification (`wxNotificationMessage`).
pub struct NotificationMessage {
    title: String,
    message: String,
    icon: BalloonIcon,
}

impl NotificationMessage {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            message: String::new(),
            icon: BalloonIcon::Info,
        }
    }

    pub fn set_message(&mut self, message: &str) -> &mut Self {
        self.message = message.to_string();
        self
    }

    pub fn set_icon(&mut self, icon: BalloonIcon) -> &mut Self {
        self.icon = icon;
        self
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    /// Show via the frame's tray icon balloon, if one is registered.
    pub fn show(&self, frame: &Frame, tray: &IconTray) {
        let _ = frame;
        tray.show_balloon(&self.title, &self.message, self.icon);
    }
}
