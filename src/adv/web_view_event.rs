//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! WebView navigation event (`wxWebViewEvent`).

/// WebView load state (`wxWebViewEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebViewEventKind {
    NavigationRequested,
    NavigationComplete,
    TitleChanged,
    NewWindow,
    Error,
}

/// Notification from an embedded browser (`wxWebViewEvent`).
#[derive(Debug, Clone)]
pub struct WebViewEvent {
    pub kind: WebViewEventKind,
    pub url: String,
}

impl WebViewEvent {
    pub fn new(kind: WebViewEventKind, url: impl Into<String>) -> Self {
        Self {
            kind,
            url: url.into(),
        }
    }
}
