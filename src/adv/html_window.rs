//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! HTML viewer (`wxHtmlWindow`) — renders stripped HTML as static text.

use crate::controls::static_text::StaticText;
use crate::core::widget::{WidgetRef, Window};

#[derive(Clone)]
pub struct HtmlWindow {
    label: StaticText,
    html: String,
}

impl HtmlWindow {
    pub fn new<W: Window>(parent: &W) -> Self {
        Self {
            label: StaticText::new(parent, ""),
            html: String::new(),
        }
    }

    pub fn set_page(&mut self, html: &str) {
        self.html = html.to_string();
        let plain = html
            .replace("<br>", "\n")
            .replace("<br/>", "\n")
            .replace("<p>", "")
            .replace("</p>", "\n");
        let stripped: String = plain
            .chars()
            .scan(false, |in_tag, c| {
                match c {
                    '<' => {
                        *in_tag = true;
                        Some(None)
                    }
                    '>' => {
                        *in_tag = false;
                        Some(None)
                    }
                    _ if *in_tag => Some(None),
                    other => Some(Some(other)),
                }
            })
            .flatten()
            .collect();
        self.label.set_label(&stripped);
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.label.as_widget_ref()
    }
}
