//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Search field (`wxSearchCtrl`) — `EDIT` with cue banner.

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};
#[cfg(target_os = "windows")]
use crate::platform::win32::read_window_text;
use crate::window::frame::Frame;

use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[cfg(target_os = "windows")]
const ES_AUTOHSCROLL: u32 = 0x0080;
#[cfg(target_os = "windows")]
const EM_SETCUEBANNER: u32 = 0x1501;
#[cfg(target_os = "windows")]
const EN_CHANGE: u32 = 0x0300;

struct SearchCtrlInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    rect: Rect,
    visible: bool,
    placeholder: String,
}

#[derive(Clone)]
pub struct SearchCtrl {
    inner: Rc<RefCell<SearchCtrlInner>>,
}

impl SearchCtrl {
    pub fn new<W: Window>(parent: &W, placeholder: &str) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        let hwnd = unsafe {
            let parent_hwnd = parent.hwnd();
            let wide_class = to_wide("EDIT");
            let hwnd = CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_AUTOHSCROLL,
                0,
                0,
                220,
                24,
                parent_hwnd,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            if !hwnd.is_null() {
                let cue = to_wide(placeholder);
                SendMessageW(hwnd, EM_SETCUEBANNER, 1, cue.as_ptr() as isize);
            }
            hwnd
        };

        #[cfg(not(target_os = "windows"))]
        {
            let _ = parent;
        }

        SearchCtrl {
            inner: Rc::new(RefCell::new(SearchCtrlInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                rect: Rect::new(0, 0, 220, 24),
                visible: true,
                placeholder: placeholder.to_string(),
            })),
        }
    }

    pub fn value(&self) -> String {
        #[cfg(target_os = "windows")]
        {
            read_window_text(self.inner.borrow().hwnd)
        }
        #[cfg(not(target_os = "windows"))]
        {
            String::new()
        }
    }

    pub fn set_value(&self, text: &str) {
        #[cfg(target_os = "windows")]
        unsafe {
            let wide = to_wide(text);
            SetWindowTextW(self.inner.borrow().hwnd, wide.as_ptr());
        }
        #[cfg(not(target_os = "windows"))]
        let _ = text;
    }

    pub fn clear(&self) {
        self.set_value("");
    }

    pub fn on_change<F: FnMut() + 'static>(&self, frame: &Frame, f: F) {
        let id = self.inner.borrow().id;
        frame.register_command_handler(id, Box::new(f));
    }

    pub fn on_search<F: FnMut(String) + 'static>(&self, frame: &Frame, mut f: F) {
        let ctrl = self.clone();
        self.on_change(frame, move || {
            f(ctrl.value());
        });
    }

    /// Text change with [`SearchCtrlEvent`] payload (`wxSearchCtrlEvent`).
    pub fn on_search_event<F: FnMut(&crate::SearchCtrlEvent) + 'static>(
        &self,
        frame: &Frame,
        mut f: F,
    ) {
        let ctrl = self.clone();
        self.on_change(frame, move || {
            f(&crate::SearchCtrlEvent::new(ctrl.value()));
        });
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

impl Widget for SearchCtrlInner {
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
