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
    id: u16,
    label: String,
    url: String,
    rect: Rect,
    visible: bool,
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
            let parent_hwnd = parent.hwnd();
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

        #[cfg(not(target_os = "windows"))]
        let _ = (parent, &markup);

        HyperlinkCtrl {
            inner: Rc::new(RefCell::new(HyperlinkCtrlInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                label: label.to_string(),
                url: url.to_string(),
                rect: Rect::new(0, 0, 200, 20),
                visible: true,
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
        #[cfg(not(target_os = "windows"))]
        let _ = markup;
    }

    /// Fires when the user activates the link (`NM_CLICK`).
    pub fn on_click<F: FnMut() + 'static>(&self, frame: &Frame, mut f: F) {
        let id = self.inner.borrow().id;
        frame.register_notify_handler(id, Box::new(move |code| {
            #[cfg(target_os = "windows")]
            if code as i32 == NM_CLICK {
                f();
            }
            #[cfg(not(target_os = "windows"))]
            let _ = code;
        }));
    }

    /// Link activated with [`HyperlinkEvent`] (`wxHyperlinkEvent`).
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
        #[cfg(not(target_os = "windows"))]
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
    }

    fn set_size(&mut self, w: u32, h: u32) {
        self.rect.width = w;
        self.rect.height = h;
        #[cfg(target_os = "windows")]
        unsafe {
            MoveWindow(self.hwnd, self.rect.x, self.rect.y, w as i32, h as i32, 1);
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
    }

    fn is_enabled(&self) -> bool {
        true
    }

    fn set_enabled(&mut self, _enabled: bool) {}
}
