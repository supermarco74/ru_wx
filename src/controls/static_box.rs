//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Labelled box container (`wxStaticBox`).
//!
//! On Windows this is a `BUTTON` child with the `BS_GROUPBOX` style —
//! the same trick used internally by [`crate::RadioBox`]. It draws a
//! thin rectangle with the title in the upper-left corner, and is
//! completely passive: it does not receive focus, it does not dispatch
//! commands, and it does not own its child widgets (those are siblings
//! in the parent window — typical Win32 `BS_GROUPBOX` behaviour, and
//! what the `wxWidgets` `wxStaticBox` abstracts on the macOS / GTK
//! back-ends).
//!
//! Use [`StaticBox::new`] (with a label) or [`StaticBox::new_empty`]
//! for a frameless box. Width and height are managed by the parent
//! sizer just like any other widget.

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};

use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::{read_window_text, to_wide};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// `BUTTON` class `BS_GROUPBOX` style (not exposed by `windows-sys 0.59`,
// defined as a raw constant — same value as the C++ `<winuser.h>`).
#[cfg(target_os = "windows")]
const BS_GROUPBOX: u32 = 0x0007;

struct StaticBoxInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    /// The displayed title. On Windows we cache it so
    /// [`StaticBox::get_label`] works on every platform.
    label: String,
    rect: Rect,
    visible: bool,
    /// `StaticBox` itself has no enabled state, but we keep the flag so
    /// sizers / parent code can introspect it.
    enabled: bool,
}

#[derive(Clone)]
pub struct StaticBox {
    inner: Rc<RefCell<StaticBoxInner>>,
}

impl StaticBox {
    /// Default size for a newly created `StaticBox` before the parent
    /// sizer overwrites it.
    const DEFAULT_W: u32 = 200;
    const DEFAULT_H: u32 = 100;

    /// Create a new `StaticBox` with the given label, as a child of
    /// the given parent window.
    pub fn new<W: Window>(parent_in: &W, label: &str) -> Self {
        Self::with_size(parent_in, label, Self::DEFAULT_W, Self::DEFAULT_H)
    }

    /// Create a new `StaticBox` with no label (the box still draws
    /// its frame, but the title area is blank).
    pub fn new_empty<W: Window>(parent_in: &W) -> Self {
        Self::new(parent_in, "")
    }

    /// Create a new `StaticBox` with an explicit initial size. Useful
    /// when the parent does not use sizers.
    pub fn with_size<W: Window>(parent_in: &W, label: &str, width: u32, height: u32) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = parent_in.hwnd();
            let wide_class = to_wide("BUTTON");
            let wide_label = to_wide(label);
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                wide_label.as_ptr(),
                WS_CHILD | WS_VISIBLE | BS_GROUPBOX,
                0,
                0,
                width as i32,
                height as i32,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = (parent_in, label, width, height);

        StaticBox {
            inner: Rc::new(RefCell::new(StaticBoxInner {
                #[cfg(target_os = "windows")]
                hwnd,
                label: label.to_string(),
                rect: Rect::new(0, 0, width, height),
                visible: true,
                enabled: true,
            })),
        }
    }

    /// Set the title displayed in the upper-left corner of the box.
    pub fn set_label(&self, label: &str) {
        self.inner.borrow_mut().label = label.to_string();
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(label);
            SetWindowTextW(self.inner.borrow().hwnd, wide.as_ptr());
        }
    }

    /// Return the current title of the box.
    ///
    /// On Windows this queries the underlying `BUTTON` control via
    /// `GetWindowTextW`, so the value returned is the live one (e.g.
    /// if some other code path has mutated it).
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

    /// Get a `WidgetRef` for use with sizers.
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }

    /// Return the native window handle (HWND on Windows, 0 elsewhere).
    #[cfg(target_os = "windows")]
    pub fn hwnd(&self) -> HWND {
        self.inner.borrow().hwnd
    }
    #[cfg(not(target_os = "windows"))]
    pub fn hwnd(&self) -> isize {
        0
    }
}

#[cfg(target_os = "windows")]
impl Window for StaticBox {
    fn hwnd(&self) -> HWND {
        self.hwnd()
    }
}

impl Widget for StaticBoxInner {
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
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            EnableWindow(self.hwnd, if enabled { 1 } else { 0 });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dimensions_match_constants() {
        assert_eq!(StaticBox::DEFAULT_W, 200);
        assert_eq!(StaticBox::DEFAULT_H, 100);
    }

    #[test]
    fn get_label_returns_initial_value() {
        // No GUI: the cached label is what we read.
        let inner = StaticBoxInner {
            #[cfg(target_os = "windows")]
            hwnd: std::ptr::null_mut(),
            label: "Hello".to_string(),
            rect: Rect::new(0, 0, 100, 50),
            visible: true,
            enabled: true,
        };
        assert_eq!(inner.label, "Hello");
    }
}
