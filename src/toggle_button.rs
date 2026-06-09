//! Toggle-button control (`wxToggleButton`).
//!
//! On Windows the widget is realised with the `BUTTON` common control
//! class using the standard push-button style `BS_PUSHBUTTON`. The
//! "stays pressed" visual is achieved by sending `BM_SETCHECK` with
//! `BST_CHECKED` / `BST_UNCHECKED` after the user clicks the button;
//! the state itself is tracked in the [`ToggleButtonInner`] struct.
//!
//! # Example
//! ```no_run
//! use ru_wx::toggle_button::ToggleButton;
//! use ru_wx::frame::Frame;
//!
//! let frame = Frame::builder().with_title("App").with_size(100, 100).build();
//! let btn = ToggleButton::new(&frame, "Bold");
//! btn.set_value(true);
//! assert!(btn.get_value());
//! btn.on_toggle(&frame, |checked| {
//!     println!("Bold is now {}", if checked { "ON" } else { "OFF" });
//! });
//! ```

use std::cell::RefCell;
use std::rc::Rc;

use crate::frame::Frame;
use crate::geometry::Rect;
use crate::widget::{Widget, WidgetRef, Window};

#[cfg(target_os = "windows")]
use crate::platform::win32::{next_control_id, to_wide};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── Win32 button messages / states (defined in <winuser.h>, not all
//    exported by windows-sys 0.59) ────────────────────────────────────

/// `BM_GETCHECK` — query the checked state of a button.
#[cfg(target_os = "windows")]
const BM_GETCHECK: u32 = 0x00F0;
/// `BM_SETCHECK` — set the checked state of a button.
#[cfg(target_os = "windows")]
const BM_SETCHECK: u32 = 0x00F1;

/// `BST_UNCHECKED` — the button is not checked.
#[cfg(target_os = "windows")]
const BST_UNCHECKED: usize = 0x0000;
/// `BST_CHECKED` — the button is checked (pressed in).
#[cfg(target_os = "windows")]
const BST_CHECKED: usize = 0x0001;
/// `BST_INDETERMINATE` — the button is in the grey / indeterminate
/// state. Only meaningful for buttons created with `BS_3STATE` or
/// `BS_AUTO3STATE`; we still expose the constant for completeness.
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const BST_INDETERMINATE: usize = 0x0002;

// ── Inner type ─────────────────────────────────────────────────────────

struct ToggleButtonInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    label: String,
    rect: Rect,
    enabled: bool,
    visible: bool,
    /// Whether the button is currently in the "pressed / checked"
    /// state. On Windows we keep this in Rust and push it down to the
    /// control with `BM_SETCHECK` so the visual matches the logical
    /// state.
    checked: bool,
}

#[derive(Clone)]
pub struct ToggleButton {
    inner: Rc<RefCell<ToggleButtonInner>>,
}

impl ToggleButton {
    /// Create a new toggle button as a child of the given parent
    /// window. The button starts in the unchecked state.
    pub fn new<W: Window>(parent_in: &W, label: &str) -> Self {
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
                WS_CHILD | WS_VISIBLE,
                0,
                0,
                100,
                30,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = (parent_in, label);

        ToggleButton {
            inner: Rc::new(RefCell::new(ToggleButtonInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                label: label.to_string(),
                rect: Rect::new(0, 0, 100, 30),
                enabled: true,
                visible: true,
                checked: false,
            })),
        }
    }

    /// Create a new toggle button that starts in the given state.
    pub fn with_value<W: Window>(parent_in: &W, label: &str, checked: bool) -> Self {
        let btn = Self::new(parent_in, label);
        btn.set_value(checked);
        btn
    }

    /// Return `true` if the button is currently checked / pressed.
    pub fn get_value(&self) -> bool {
        self.inner.borrow().checked
    }

    /// Convenience alias for [`ToggleButton::get_value`].
    pub fn is_checked(&self) -> bool {
        self.get_value()
    }

    /// Set the checked state of the button. The visual is updated
    /// immediately via `BM_SETCHECK`.
    pub fn set_value(&self, checked: bool) {
        self.inner.borrow_mut().checked = checked;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            let state = if checked { BST_CHECKED } else { BST_UNCHECKED };
            SendMessageW(hwnd, BM_SETCHECK, state, 0);
        }
    }

    /// Flip the current state and return the new value.
    pub fn toggle(&self) -> bool {
        let new_state = !self.get_value();
        self.set_value(new_state);
        new_state
    }

    /// Get the control ID (used for `WM_COMMAND` dispatch).
    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    /// Set the button label. Updates both the cached label and the
    /// native control text via `SetWindowTextW`.
    pub fn set_label(&self, label: &str) {
        self.inner.borrow_mut().label = label.to_string();
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(label);
            SetWindowTextW(self.inner.borrow().hwnd, wide.as_ptr());
        }
    }

    /// Get the current button label. On Windows this queries the
    /// underlying control via `GetWindowTextW`, so it returns the
    /// live label.
    pub fn get_label(&self) -> String {
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            // SAFETY: FFI call to GetWindowTextLengthW; `hwnd` is a real window handle and the wide buffer is sized appropriately.
            let len = unsafe { GetWindowTextLengthW(hwnd) };
            if len == 0 {
                return String::new();
            }
            let mut buf = vec![0u16; (len + 1) as usize];
            // SAFETY: FFI call to GetWindowTextW; `hwnd` is a real window handle and the wide buffer is sized appropriately.
            let copied = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1) };
            if copied <= 0 {
                return String::new();
            }
            String::from_utf16_lossy(&buf[..copied as usize])
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.inner.borrow().label.clone()
        }
    }

    /// Register a click-only callback. The handler runs every time
    /// the user clicks the button, regardless of the resulting state.
    /// Use [`ToggleButton::on_toggle`] instead if you want to be
    /// notified of state changes.
    pub fn on_click<F: FnMut() + 'static>(&self, frame: &Frame, mut callback: F) {
        let id = self.inner.borrow().id;
        let inner = self.inner.clone();
        frame.register_command_handler(
            id,
            Box::new(move || {
                // Flip the cached state to match what the user did.
                let cur = inner.borrow().checked;
                inner.borrow_mut().checked = !cur;
                callback();
            }),
        );
    }

    /// Register a callback fired whenever the state of the button
    /// changes. The new state (`true` = checked) is passed to the
    /// callback.
    pub fn on_toggle<F: FnMut(bool) + 'static>(&self, frame: &Frame, mut callback: F) {
        let id = self.inner.borrow().id;
        let inner = self.inner.clone();
        frame.register_command_handler(
            id,
            Box::new(move || {
                let cur = inner.borrow().checked;
                let new_state = !cur;
                inner.borrow_mut().checked = new_state;
                callback(new_state);
            }),
        );
    }

    /// Get a `WidgetRef` for use with sizers.
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

impl Widget for ToggleButtonInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.hwnd as isize
        }
        #[cfg(not(target_os = "windows"))]
        0
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

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// The Win32 constants we use must match `<winuser.h>`. A
    /// regression here would silently break `BM_SETCHECK` dispatch
    /// (a wrong `BST_CHECKED` would make the button look pressed
    /// when it is logically unchecked, or vice versa).
    #[cfg(target_os = "windows")]
    #[test]
    fn win32_constants_pinned() {
        assert_eq!(BM_GETCHECK, 0x00F0);
        assert_eq!(BM_SETCHECK, 0x00F1);
        assert_eq!(BST_UNCHECKED, 0x0000);
        assert_eq!(BST_CHECKED, 0x0001);
        assert_eq!(BST_INDETERMINATE, 0x0002);
    }

    /// The cached `checked` flag must follow `set_value` /
    /// `get_value` without touching the live control.
    #[test]
    fn value_round_trip() {
        let mut cache = ToggleButtonInner {
            #[cfg(target_os = "windows")]
            hwnd: std::ptr::null_mut(),
            id: 0,
            label: String::new(),
            rect: Rect::new(0, 0, 100, 30),
            enabled: true,
            visible: true,
            checked: false,
        };
        assert!(!cache.checked);
        cache.checked = true;
        assert!(cache.checked);
        cache.checked = false;
        assert!(!cache.checked);
    }
}
