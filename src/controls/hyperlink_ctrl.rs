//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Clickable hyperlink (`wxHyperlinkCtrl`) — Win32 `SysLink`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};
use crate::window::frame::Frame;

use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[cfg(target_os = "windows")]
const NM_CLICK: i32 = -2;
#[cfg(target_os = "windows")]
const WS_EX_NOPARENTNOTIFY: u32 = 0x0000_0004;

fn link_markup(label: &str, url: &str) -> String {
    format!(r#"<a href="{url}">{label}</a>"#)
}

struct HyperlinkCtrlInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(target_os = "macos")]
    stub_handle: crate::platform::NativeHandle,
    #[cfg(target_os = "macos")]
    parent_handle: crate::platform::NativeHandle,
    id: u16,
    label: String,
    url: String,
    rect: Rect,
    visible: bool,
    enabled: bool,
}

#[derive(Clone)]
pub struct HyperlinkCtrl {
    inner: Rc<RefCell<HyperlinkCtrlInner>>,
}

impl HyperlinkCtrl {
    /// Create a `SysLink` control showing `label` pointing to `url`.
    pub fn new<W: Window>(parent: &W, label: &str, url: &str) -> Self {
        let id = next_control_id();
        let markup = link_markup(label, url);

        #[cfg(target_os = "windows")]
        let hwnd = unsafe {
            let parent_hwnd = crate::platform::window_handle_as_hwnd(parent);
            let wide = to_wide(&markup);
            let class = to_wide("SysLink");
            CreateWindowExW(
                WS_EX_NOPARENTNOTIFY,
                class.as_ptr(),
                wide.as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP,
                0,
                0,
                200,
                20,
                parent_hwnd,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = (parent, &markup);

        #[cfg(target_os = "macos")]
        let parent_handle = crate::platform::window_handle(parent);
        #[cfg(target_os = "macos")]
        let stub_handle = crate::platform::appkit::create_hyperlink(
            parent_handle,
            label,
            url,
            id,
            Rect::new(0, 0, 200, 20),
        );

        HyperlinkCtrl {
            inner: Rc::new(RefCell::new(HyperlinkCtrlInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(target_os = "macos")]
                stub_handle,
                #[cfg(target_os = "macos")]
                parent_handle,
                id,
                label: label.to_string(),
                url: url.to_string(),
                rect: Rect::new(0, 0, 200, 20),
                visible: true,
                enabled: true,
            })),
        }
    }

    pub fn url(&self) -> String {
        self.inner.borrow().url.clone()
    }

    pub fn set_url(&self, url: &str) {
        let label = self.inner.borrow().label.clone();
        self.inner.borrow_mut().url = url.to_string();
        self.set_markup(&link_markup(&label, url));
    }

    pub fn set_label(&self, label: &str) {
        let url = self.inner.borrow().url.clone();
        self.inner.borrow_mut().label = label.to_string();
        self.set_markup(&link_markup(label, &url));
    }

    fn set_markup(&self, markup: &str) {
        #[cfg(target_os = "windows")]
        unsafe {
            let wide = to_wide(markup);
            SetWindowTextW(self.inner.borrow().hwnd, wide.as_ptr());
        }
        #[cfg(target_os = "macos")]
        {
            let inner = self.inner.borrow();
            crate::platform::appkit::hyperlink_update(inner.stub_handle, &inner.label, &inner.url);
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = markup;
    }

    /// Fires when the user activates the link (`NM_CLICK`).
    pub fn on_click<F: FnMut() + 'static>(&self, frame: &Frame, f: F) {
        let id = self.inner.borrow().id;
        #[cfg(target_os = "windows")]
        {
            let mut handler = f;
            frame.register_notify_handler(
                id,
                Box::new(move |code| {
                    if code as i32 == NM_CLICK {
                        handler();
                    }
                }),
            );
        }
        #[cfg(target_os = "macos")]
        {
            frame.register_command_handler(id, Box::new(f));
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            let _ = (frame, f);
        }
    }

    /// Link activated with [`HyperlinkEvent`](crate::HyperlinkEvent) (`wxHyperlinkEvent`).
    pub fn on_link<F: FnMut(&crate::HyperlinkEvent) + 'static>(&self, frame: &Frame, mut f: F) {
        let url = self.inner.borrow().url.clone();
        self.on_click(frame, move || {
            f(&crate::HyperlinkEvent::new(&url));
        });
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

impl Widget for HyperlinkCtrlInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.hwnd as isize
        }
        #[cfg(target_os = "macos")]
        {
            self.stub_handle
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            0
        }
    }

    fn set_position(&mut self, x: i32, y: i32) {
        self.rect.x = x;
        self.rect.y = y;
        #[cfg(target_os = "windows")]
        unsafe {
            MoveWindow(
                self.hwnd,
                x,
                y,
                self.rect.width as i32,
                self.rect.height as i32,
                1,
            );
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::macos_widget_set_frame(
                self.parent_handle,
                self.stub_handle,
                self.rect,
            );
        }
    }

    fn set_size(&mut self, w: u32, h: u32) {
        self.rect.width = w;
        self.rect.height = h;
        #[cfg(target_os = "windows")]
        unsafe {
            MoveWindow(self.hwnd, self.rect.x, self.rect.y, w as i32, h as i32, 1);
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::macos_widget_set_frame(
                self.parent_handle,
                self.stub_handle,
                self.rect,
            );
        }
    }

    fn rect(&self) -> Rect {
        self.rect
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
        #[cfg(target_os = "windows")]
        unsafe {
            ShowWindow(self.hwnd, if visible { SW_SHOW } else { SW_HIDE });
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::macos_widget_set_visible(self.stub_handle, visible);
        }
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            EnableWindow(self.hwnd, if enabled { 1 } else { 0 });
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::macos_widget_set_enabled(self.stub_handle, enabled);
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = enabled;
    }
}
