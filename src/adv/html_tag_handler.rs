//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! HTML tag handler (`wxHtmlTagHandler`).

/// Parses and handles a single HTML tag (`wxHtmlTagHandler`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlTagHandler {
    pub tag: String,
    pub attributes: Vec<(String, String)>,
}

impl HtmlTagHandler {
    pub fn new(tag: &str) -> Self {
        Self {
            tag: tag.to_string(),
            attributes: Vec::new(),
        }
    }

    pub fn with_attribute(mut self, name: &str, value: &str) -> Self {
        self.attributes.push((name.to_string(), value.to_string()));
        self
    }

    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    pub fn render_opening(&self) -> String {
        if self.attributes.is_empty() {
            return format!("<{}>", self.tag);
        }
        let attrs = self
            .attributes
            .iter()
            .map(|(k, v)| format!("{k}=\"{v}\""))
            .collect::<Vec<_>>()
            .join(" ");
        format!("<{} {attrs}>", self.tag)
    }
}
