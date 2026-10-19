//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! HTML viewer (`wxHtmlWindow`) — `WKWebView` on macOS, stripped text elsewhere.

use std::cell::RefCell;
use std::rc::Rc;

use crate::controls::static_text::StaticText;
use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};

#[cfg(target_os = "macos")]
struct HtmlWindowInner {
    stub_handle: crate::platform::NativeHandle,
    parent_handle: crate::platform::NativeHandle,
    html: String,
    rect: Rect,
    visible: bool,
}

#[cfg(not(target_os = "macos"))]
struct HtmlWindowInner {
    label: StaticText,
    html: String,
}

#[derive(Clone)]
pub struct HtmlWindow {
    inner: Rc<RefCell<HtmlWindowInner>>,
}

impl HtmlWindow {
    pub fn new<W: Window>(parent: &W) -> Self {
        #[cfg(target_os = "macos")]
        {
            let parent_handle = crate::platform::window_handle(parent);
            let stub_handle =
                crate::platform::appkit::create_web_view(parent_handle, Rect::new(0, 0, 400, 300));
            Self {
                inner: Rc::new(RefCell::new(HtmlWindowInner {
                    stub_handle,
                    parent_handle,
                    html: String::new(),
                    rect: Rect::new(0, 0, 400, 300),
                    visible: true,
                })),
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            Self {
                inner: Rc::new(RefCell::new(HtmlWindowInner {
                    label: StaticText::new(parent, ""),
                    html: String::new(),
                })),
            }
        }
    }

    pub fn set_page(&mut self, html: &str) {
        self.inner.borrow_mut().html = html.to_string();
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::webview_load_html(self.inner.borrow().stub_handle, html);
        }
        #[cfg(not(target_os = "macos"))]
        {
            let plain = html
                .replace("<br>", "\n")
                .replace("<br/>", "\n")
                .replace("<p>", "")
                .replace("</p>", "\n");
            let stripped: String = plain
                .chars()
                .scan(false, |in_tag, c| match c {
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
                })
                .flatten()
                .collect();
            self.inner.borrow().label.set_label(&stripped);
        }
    }

    /// Load a remote URL (macOS native WebKit; other platforms use HTTP fetch + `set_page`).
    pub fn load_url(&mut self, url: &str) {
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::webview_load_url(self.inner.borrow().stub_handle, url);
            self.inner.borrow_mut().html = url.to_string();
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = url;
        }
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

#[cfg(target_os = "macos")]
impl Widget for HtmlWindowInner {
    fn native_handle(&self) -> isize {
        self.stub_handle
    }

    fn set_position(&mut self, x: i32, y: i32) {
        self.rect.x = x;
        self.rect.y = y;
        crate::platform::appkit::macos_widget_set_frame(
            self.parent_handle,
            self.stub_handle,
            self.rect,
        );
    }

    fn set_size(&mut self, width: u32, height: u32) {
        self.rect.width = width;
        self.rect.height = height;
        crate::platform::appkit::macos_widget_set_frame(
            self.parent_handle,
            self.stub_handle,
            self.rect,
        );
    }

    fn rect(&self) -> Rect {
        self.rect
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
        crate::platform::appkit::macos_widget_set_visible(self.stub_handle, visible);
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn set_enabled(&mut self, enabled: bool) {
        crate::platform::appkit::macos_widget_set_enabled(self.stub_handle, enabled);
    }

    fn is_enabled(&self) -> bool {
        true
    }
}

#[cfg(not(target_os = "macos"))]
impl Widget for HtmlWindowInner {
    fn native_handle(&self) -> isize {
        self.label.as_widget_ref().borrow().native_handle()
    }

    fn set_position(&mut self, x: i32, y: i32) {
        self.label.as_widget_ref().borrow_mut().set_position(x, y);
    }

    fn set_size(&mut self, width: u32, height: u32) {
        self.label
            .as_widget_ref()
            .borrow_mut()
            .set_size(width, height);
    }

    fn rect(&self) -> Rect {
        self.label.as_widget_ref().borrow().rect()
    }

    fn set_visible(&mut self, visible: bool) {
        self.label.as_widget_ref().borrow_mut().set_visible(visible);
    }

    fn is_visible(&self) -> bool {
        self.label.as_widget_ref().borrow().is_visible()
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.label.as_widget_ref().borrow_mut().set_enabled(enabled);
    }

    fn is_enabled(&self) -> bool {
        self.label.as_widget_ref().borrow().is_enabled()
    }
}
