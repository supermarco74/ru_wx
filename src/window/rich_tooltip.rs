//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Multi-line tooltip (`wxRichToolTip`).

use crate::core::tooltip::ToolTip;
use crate::core::widget::WidgetRef;

/// Title + message tooltip (`wxRichToolTip`).
pub struct RichToolTip {
    title: String,
    message: String,
}

impl RichToolTip {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            message: String::new(),
        }
    }

    pub fn set_message(&mut self, message: &str) -> &mut Self {
        self.message = message.to_string();
        self
    }

    pub fn show_for(&self, widget: &WidgetRef) {
        let text = if self.message.is_empty() {
            self.title.clone()
        } else {
            format!("{}\n{}", self.title, self.message)
        };
        let tip = ToolTip::new(&text);
        tip.attach(widget);
    }
}
