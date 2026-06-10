//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Embedded browser (`wxWebView`) — stub until WebView2 integration.

use crate::controls::static_text::StaticText;
use crate::core::widget::{WidgetRef, Window};

#[derive(Clone)]
pub struct WebView {
    placeholder: StaticText,
    url: String,
}

impl WebView {
    pub fn new<W: Window>(parent: &W) -> Self {
        Self {
            placeholder: StaticText::new(parent, "(WebView stub)"),
            url: String::new(),
        }
    }

    pub fn load_url(&mut self, url: &str) {
        self.url = url.to_string();
        self.placeholder
            .set_label(&format!("WebView stub: {url}"));
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.placeholder.as_widget_ref()
    }
}
