//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Rich text editor (`wxRichTextCtrl`) — multiline `TextCtrl` wrapper.

use crate::controls::text_ctrl::TextCtrl;
use crate::core::widget::{WidgetRef, Window};

#[derive(Clone)]
pub struct RichTextCtrl {
    text: TextCtrl,
}

impl RichTextCtrl {
    pub fn new<W: Window>(parent: &W) -> Self {
        let text = TextCtrl::multiline(parent, "");
        Self { text }
    }

    pub fn set_value(&self, value: &str) {
        self.text.set_value(value);
    }

    pub fn value(&self) -> String {
        self.text.get_value()
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.text.as_widget_ref()
    }
}
