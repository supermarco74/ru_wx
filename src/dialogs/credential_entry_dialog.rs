//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Credential entry dialog (`wxCredentialEntryDialog`).

use crate::window::frame::Frame;

/// Username/password prompt (`wxCredentialEntryDialog`).
pub struct CredentialEntryDialog {
    title: String,
    message: String,
    username: String,
    password: String,
}

impl CredentialEntryDialog {
    pub fn new(title: &str, message: &str) -> Self {
        Self {
            title: title.to_string(),
            message: message.to_string(),
            username: String::new(),
            password: String::new(),
        }
    }

    pub fn with_defaults(mut self, username: &str, password: &str) -> Self {
        self.username = username.to_string();
        self.password = password.to_string();
        self
    }

    pub fn show_modal(self, _frame: &Frame) -> Option<(String, String)> {
        let _ = (self.title, self.message);
        if self.username.is_empty() && self.password.is_empty() {
            None
        } else {
            Some((self.username, self.password))
        }
    }
}
