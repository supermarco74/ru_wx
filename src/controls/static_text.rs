//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Read-only label control (`wxStaticText`).
//!
//! On Windows the widget is a `STATIC` child with style `SS_LEFT`.
//! Use [`StaticText::new`] to create one and [`StaticText::set_label`]
//! / [`StaticText::get_label`] to update / read the displayed text.

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::font::Font;
use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};

#[cfg(target_os = "windows")]
use crate::platform::win32::{read_window_text, to_wide};
use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

struct StaticTextInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(not(target_os = "windows"))]
    stub_handle: isize,
    label: String,
    rect: Rect,
    visible: bool,
}

#[derive(Clone)]
pub struct StaticText {
    inner: Rc<RefCell<StaticTextInner>>,
}

impl StaticText {
    /// Create a new static text label as a child of the given parent window.
    pub fn new<W: Window>(parent_in: &W, text: &str) -> Self {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = parent_in.hwnd();
            let wide_text = to_wide(text);
            let wide_class = to_wide("STATIC");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                wide_text.as_ptr(),
                WS_CHILD | WS_VISIBLE,
                0,
                0,
                200,
                20,
                parent,
                next_control_id() as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = parent_in;

        StaticText {
            inner: Rc::new(RefCell::new(StaticTextInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(not(target_os = "windows"))]
                stub_handle: crate::platform::stub_backend::alloc_widget_handle(),
                label: text.to_string(),
                rect: Rect::new(0, 0, 200, 20),
                visible: true,
            })),
        }
    }

    /// Set the text content
    pub fn set_label(&self, text: &str) {
        self.inner.borrow_mut().label = text.to_string();
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(text);
            SetWindowTextW(self.inner.borrow().hwnd, wide.as_ptr());
        }
    }

    /// Get the current text content of the control.
    ///
    /// On Windows this queries the underlying `STATIC` control via
    /// `GetWindowTextW`, so the value returned is the live one (e.g.
    /// if a parent sizer has mutated the text).
    pub fn get_label(&self) -> String {
        #[cfg(target_os = "windows")]
        {
            read_window_text(self.inner.borrow().hwnd)
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.inner.borrow().label.clone()
        }
    }

    /// Apply a custom font to the control.
    ///
    /// On Windows this sends `WM_SETFONT` with `lParam = 1` so the
    /// control repaints immediately.
    pub fn set_font(&self, font: &Font) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(
                self.inner.borrow().hwnd,
                WM_SETFONT,
                font.hfont() as usize,
                1,
            );
        }
        #[cfg(not(target_os = "windows"))]
        let _ = font;
    }

    /// Get a WidgetRef for use with sizers
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

impl Widget for StaticTextInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.hwnd as isize
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.stub_handle
        }
    }

    fn set_position(&mut self, x: i32, y: i32) {
        self.rect.x = x;
        self.rect.y = y;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
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
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
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
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            ShowWindow(self.hwnd, if visible { SW_SHOW } else { SW_HIDE });
        }
    }

    fn is_enabled(&self) -> bool {
        true
    }
    fn set_enabled(&mut self, _enabled: bool) {}
}
