//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Embedded browser (`wxWebView`) — WebKit on macOS, HTTP fetch fallback elsewhere.

use std::cell::RefCell;
use std::rc::Rc;

use crate::adv::html_window::HtmlWindow;
use crate::adv::web_view_event::{WebViewEvent, WebViewEventKind};
use crate::core::widget::{WidgetRef, Window};

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
use crate::net::WebRequest;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
use std::path::Path;

/// Replaceable navigation callback slot.
type WebViewEventHandler = Rc<RefCell<Option<Box<dyn FnMut(&WebViewEvent)>>>>;

pub struct WebView {
    html: HtmlWindow,
    url: String,
    on_event: WebViewEventHandler,
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    native_handle: crate::platform::NativeHandle,
}

impl WebView {
    pub fn new<W: Window>(parent: &W) -> Self {
        let html = HtmlWindow::new(parent);
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        let native_handle = html.as_widget_ref().borrow().native_handle();
        Self {
            html,
            url: String::new(),
            on_event: Rc::new(RefCell::new(None)),
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            native_handle,
        }
    }

    /// Register a callback for navigation / load events.
    pub fn on_event<F: FnMut(&WebViewEvent) + 'static>(&self, f: F) {
        *self.on_event.borrow_mut() = Some(Box::new(f));
        #[cfg(target_os = "macos")]
        {
            let handle = self.native_handle;
            let slot = self.on_event.clone();
            crate::platform::appkit::webview_set_event_handler(
                handle,
                Some(Box::new(move |kind, url| {
                    let event_kind = match kind {
                        crate::platform::appkit::WEBVIEW_NAV_REQUESTED => {
                            WebViewEventKind::NavigationRequested
                        }
                        crate::platform::appkit::WEBVIEW_NAV_COMPLETE => {
                            WebViewEventKind::NavigationComplete
                        }
                        crate::platform::appkit::WEBVIEW_TITLE_CHANGED => {
                            WebViewEventKind::TitleChanged
                        }
                        _ => WebViewEventKind::Error,
                    };
                    if let Some(ref mut cb) = *slot.borrow_mut() {
                        cb(&WebViewEvent::new(event_kind, url));
                    }
                })),
            );
        }
        #[cfg(target_os = "windows")]
        {
            let handle = self.native_handle;
            let slot = self.on_event.clone();
            crate::platform::win32_proprietary::webview_set_event_handler(
                handle,
                Some(Box::new(move |kind, url| {
                    let event_kind = match kind {
                        crate::platform::win32_proprietary::WEBVIEW_NAV_REQUESTED => {
                            WebViewEventKind::NavigationRequested
                        }
                        crate::platform::win32_proprietary::WEBVIEW_NAV_COMPLETE => {
                            WebViewEventKind::NavigationComplete
                        }
                        crate::platform::win32_proprietary::WEBVIEW_TITLE_CHANGED => {
                            WebViewEventKind::TitleChanged
                        }
                        _ => WebViewEventKind::Error,
                    };
                    if let Some(ref mut cb) = *slot.borrow_mut() {
                        cb(&WebViewEvent::new(event_kind, url));
                    }
                })),
            );
        }
    }

    fn fire_event(&self, kind: WebViewEventKind, url: &str) {
        if let Some(cb) = self.on_event.borrow_mut().as_mut() {
            cb(&WebViewEvent::new(kind, url));
        }
    }

    /// Load a remote or local URL.
    pub fn load_url(&mut self, url: &str) {
        self.url = url.to_string();

        #[cfg(target_os = "macos")]
        {
            self.html.load_url(url);
            return;
        }

        #[cfg(target_os = "windows")]
        {
            self.fire_event(WebViewEventKind::NavigationRequested, url);
            crate::platform::win32_proprietary::webview_load_url(self.native_handle, url);
            self.fire_event(WebViewEventKind::NavigationComplete, url);
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
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
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.html.as_widget_ref()
    }
}
