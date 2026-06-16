//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Embedded browser (`wxWebView`) — fetches HTTP(S) content and renders
//! stripped HTML via [`crate::HtmlWindow`] until WebView2 is wired in.

use std::cell::RefCell;
use std::path::Path;

use crate::adv::html_window::HtmlWindow;
use crate::adv::web_view_event::{WebViewEvent, WebViewEventKind};
use crate::core::widget::{WidgetRef, Window};
use crate::net::WebRequest;

/// Replaceable navigation callback slot.
type WebViewEventHandler = RefCell<Option<Box<dyn FnMut(&WebViewEvent)>>>;

pub struct WebView {
    html: HtmlWindow,
    url: String,
    on_event: WebViewEventHandler,
}

impl WebView {
    pub fn new<W: Window>(parent: &W) -> Self {
        Self {
            html: HtmlWindow::new(parent),
            url: String::new(),
            on_event: RefCell::new(None),
        }
    }

    /// Register a callback for navigation / load events.
    pub fn on_event<F: FnMut(&WebViewEvent) + 'static>(&self, f: F) {
        *self.on_event.borrow_mut() = Some(Box::new(f));
    }

    fn fire_event(&self, kind: WebViewEventKind, url: &str) {
        if let Some(ref mut cb) = *self.on_event.borrow_mut() {
            cb(&WebViewEvent::new(kind, url));
        }
    }

    /// Load a remote or local URL. HTTP(S) responses are fetched synchronously
    /// and rendered as stripped HTML; `file://` paths are read from disk.
    pub fn load_url(&mut self, url: &str) {
        self.url = url.to_string();
        self.fire_event(WebViewEventKind::NavigationRequested, url);
        self.html.set_page("Loading…");

        let result = if let Some(path) = url.strip_prefix("file://") {
            std::fs::read_to_string(path)
        } else if url.starts_with("http://") || url.starts_with("https://") {
            WebRequest::get(url)
                .execute()
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        } else if Path::new(url).exists() {
            std::fs::read_to_string(url)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("unsupported or missing URL: {url}"),
            ))
        };

        match result {
            Ok(body) => {
                self.html.set_page(&body);
                self.fire_event(WebViewEventKind::NavigationComplete, url);
            }
            Err(err) => {
                let msg = format!("Failed to load {url}: {err}");
                self.html.set_page(&msg);
                self.fire_event(WebViewEventKind::Error, url);
            }
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.html.as_widget_ref()
    }
}
