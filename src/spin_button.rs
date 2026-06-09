//! Spin-button control (`wxSpinButton`).
//!
//! On Windows, the spin button is realised with the common control
//! class `msctls_updown32`. Unlike [`crate::spin_ctrl::SpinCtrl`]
//! (which pairs the up-down with an `EDIT` text control as a buddy),
//! `wxSpinButton` is *only* the up/down arrows — there is no
//! associated text field. The current value is maintained entirely
//! in Rust and the control just emits a notification when the user
//! clicks the arrows or presses the up / down keys.
//!
//! # Example
//! ```no_run
//! use ru_wx::spin_button::SpinButton;
//! use ru_wx::frame::Frame;
//!
//! let frame = Frame::builder().with_title("App").with_size(100, 100).build();
//! let sb = SpinButton::new(&frame, 0, 100, 0);
//! // The closure can be `move`-captured; it doesn't need to own
//! // `sb` because `on_value_change` is a one-shot registration
//! // (the handler is stored in the frame's notify map).
//! let cb_value = std::rc::Rc::new(std::cell::RefCell::new(0));
//! let cb_value_for_closure = cb_value.clone();
//! sb.on_value_change(&frame, move || {
//!     *cb_value_for_closure.borrow_mut() += 1;
//!     println!("value changed");
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

// ── Win32 up-down constants ─────────────────────────────────────────

#[cfg(target_os = "windows")]
const UDM_SETRANGE: u32 = 0x0465;
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const UDM_GETRANGE: u32 = 0x0466;
#[cfg(target_os = "windows")]
const UDM_SETPOS: u32 = 0x0467;
#[cfg(target_os = "windows")]
const UDM_GETPOS: u32 = 0x0468;
#[cfg(target_os = "windows")]
const UDM_SETBUDDY: u32 = 0x0469;
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const UDM_SETACCEL: u32 = 0x046B;
#[cfg(target_os = "windows")]
const UDM_SETBASE: u32 = 0x046D;
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const UDM_GETBASE: u32 = 0x046E;

/// `UDS_WRAP` — when the value reaches the minimum / maximum, wrap
/// around to the other extreme.
#[cfg(target_os = "windows")]
const UDS_WRAP: u32 = 0x0001;
/// `UDS_ARROWKEYS` — the up-down control processes the up / down
/// arrow keys when it has focus.
#[cfg(target_os = "windows")]
const UDS_ARROWKEYS: u32 = 0x0020;
/// `UDS_HOTTRACK` — hot-track the arrows as the user drags.
#[cfg(target_os = "windows")]
const UDS_HOTTRACK: u32 = 0x0008;
/// `UDS_NOTHOUSANDS` — no thousands separator.
#[cfg(target_os = "windows")]
const UDS_NOTHOUSANDS: u32 = 0x0010;

// ── Inner type ───────────────────────────────────────────────────────

struct SpinButtonInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    rect: Rect,
    min: i32,
    max: i32,
    value: i32,
    wrap: bool,
    enabled: bool,
    visible: bool,
}

#[derive(Clone)]
pub struct SpinButton {
    inner: Rc<RefCell<SpinButtonInner>>,
}

impl SpinButton {
    /// Create a new spin button as a child of the given parent. The
    /// initial value is clamped to `[min, max]`.
    pub fn new<W: Window>(parent_in: &W, min: i32, max: i32, initial: i32) -> Self {
        Self::with_wrap(parent_in, min, max, initial, false)
    }

    /// Create a new spin button that wraps around at the extremes.
    pub fn with_wrap<W: Window>(
        parent_in: &W,
        min: i32,
        max: i32,
        initial: i32,
        wrap: bool,
    ) -> Self {
        let id = next_control_id();
        let initial = initial.max(min).min(max);

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = parent_in.hwnd();
            let wide_class = to_wide("msctls_updown32");
            let mut style_bits = WS_CHILD | WS_VISIBLE | UDS_ARROWKEYS | UDS_NOTHOUSANDS;
            if wrap {
                style_bits |= UDS_WRAP;
            }
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                style_bits,
                0,
                0,
                20, // narrow — the control is just the arrows
                24,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = (parent_in, wrap);

        let s = SpinButton {
            inner: Rc::new(RefCell::new(SpinButtonInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                rect: Rect::new(0, 0, 20, 24),
                min,
                max,
                value: initial,
                wrap,
                enabled: true,
                visible: true,
            })),
        };

        s.set_range(min, max);
        s.set_value(initial);
        s
    }

    /// Set the value range.
    pub fn set_range(&self, min: i32, max: i32) {
        {
            let mut inner = self.inner.borrow_mut();
            inner.min = min;
            inner.max = max;
            if inner.value < min {
                inner.value = min;
            } else if inner.value > max {
                inner.value = max;
            }
        }
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let (mut min16, mut max16) = (min, max);
            min16 = min16.clamp(0, 0xFFFF);
            if max16 < min16 {
                max16 = min16;
            }
            if max16 > 0xFFFF {
                max16 = 0xFFFF;
            }
            let range = ((max16 as u32) << 16) | (min16 as u32);
            SendMessageW(
                self.inner.borrow().hwnd,
                UDM_SETRANGE,
                0,
                range as isize,
            );
        }
    }

    /// Set the current value. The value is clamped to `[min, max]`
    /// first; pass a value outside the range and the call is a no-op
    /// for the change.
    pub fn set_value(&self, value: i32) {
        let v = {
            let mut inner = self.inner.borrow_mut();
            let v = value.max(inner.min).min(inner.max);
            inner.value = v;
            v
        };
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, UDM_SETPOS, 0, v as isize);
        }
    }

    /// Return the current value. On Windows this also refreshes the
    /// cached value from the live control.
    pub fn get_value(&self) -> i32 {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let v = unsafe { SendMessageW(self.inner.borrow().hwnd, UDM_GETPOS, 0, 0) } as i32;
            self.inner.borrow_mut().value = v;
            v
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.inner.borrow().value
        }
    }

    /// Return the current minimum.
    pub fn get_min(&self) -> i32 {
        self.inner.borrow().min
    }

    /// Return the current maximum.
    pub fn get_max(&self) -> i32 {
        self.inner.borrow().max
    }

    /// Return the current range as `(min, max)`.
    pub fn get_range(&self) -> (i32, i32) {
        (self.get_min(), self.get_max())
    }

    /// Register a callback fired when the value changes (user clicked
    /// the arrows or pressed up / down).
    pub fn on_value_change<F: FnMut() + 'static>(&self, frame: &Frame, mut callback: F) {
        let id = self.inner.borrow().id;
        let inner = self.inner.clone();
        frame.register_command_handler(
            id,
            Box::new(move || {
                #[cfg(target_os = "windows")]
                {
                    let hwnd = inner.borrow().hwnd;
                    // SAFETY: FFI call to SendMessageW; `hwnd` is a live window and `msg` / `wParam` / `lParam` are valid for that window.
                    let v = unsafe { SendMessageW(hwnd, UDM_GETPOS, 0, 0) } as i32;
                    inner.borrow_mut().value = v;
                }
                let _ = inner;
                callback();
            }),
        );
    }

    /// The control's id.
    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    /// Get a `WidgetRef` for use with sizers.
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

impl Widget for SpinButtonInner {
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

    /// The Win32 constants we use must match `<commctrl.h>`. A
    /// regression here would silently break `UDM_SETRANGE` / 
    /// `UDM_SETPOS` dispatch (a wrong `UDM_SETPOS` would, e.g., set
    /// the base instead of the position).
    #[cfg(target_os = "windows")]
    #[test]
    fn win32_constants_pinned() {
        assert_eq!(UDM_SETRANGE, 0x0465);
        assert_eq!(UDM_GETRANGE, 0x0466);
        assert_eq!(UDM_SETPOS, 0x0467);
        assert_eq!(UDM_GETPOS, 0x0468);
        assert_eq!(UDM_SETBUDDY, 0x0469);
        assert_eq!(UDM_SETACCEL, 0x046B);
        assert_eq!(UDM_SETBASE, 0x046D);
        assert_eq!(UDM_GETBASE, 0x046E);
        assert_eq!(UDS_WRAP, 0x0001);
        assert_eq!(UDS_ARROWKEYS, 0x0020);
        assert_eq!(UDS_HOTTRACK, 0x0008);
        assert_eq!(UDS_NOTHOUSANDS, 0x0010);
    }
}
