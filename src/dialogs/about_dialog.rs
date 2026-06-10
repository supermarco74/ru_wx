//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! About box (`wxAboutDialog`).

use crate::dialogs::message_box::MessageBoxResult;
use crate::dialogs::message_dialog::{MessageDialog, MessageDialogIcon, MessageDialogStyle};
use crate::window::frame::Frame;

/// Standard About dialog (`wxAboutDialog`).
pub struct AboutDialog {
    app_name: String,
    version: String,
    description: String,
    copyright: String,
}

impl AboutDialog {
    pub fn new(app_name: &str) -> Self {
        Self {
            app_name: app_name.to_string(),
            version: String::new(),
            description: String::new(),
            copyright: String::new(),
        }
    }

    pub fn with_version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self
    }

    pub fn with_description(mut self, text: &str) -> Self {
        self.description = text.to_string();
        self
    }

    pub fn with_copyright(mut self, text: &str) -> Self {
        self.copyright = text.to_string();
        self
    }

    fn message(&self) -> String {
        let mut lines = vec![self.app_name.clone()];
        if !self.version.is_empty() {
            lines.push(self.version.clone());
        }
        if !self.description.is_empty() {
            lines.push(String::new());
            lines.push(self.description.clone());
        }
        if !self.copyright.is_empty() {
            lines.push(String::new());
            lines.push(self.copyright.clone());
        }
        lines.join("\n")
    }

    pub fn show_modal(&self, frame: &Frame) -> MessageBoxResult {
        let dlg = MessageDialog::new(
            frame,
            &format!("About {}", self.app_name),
            &self.message(),
            MessageDialogStyle::Ok,
            MessageDialogIcon::Information,
        );
        dlg.show_modal()
    }
}
