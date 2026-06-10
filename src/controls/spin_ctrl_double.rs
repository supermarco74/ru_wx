//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Floating-point spin control (`wxSpinCtrlDouble`).
//!
//! On Windows, the control is realised with the same `msctls_updown32`
//! common control class as [`crate::controls::spin_ctrl::SpinCtrl`], paired with
//! an `EDIT` buddy that displays the current value. The native up-down
//! control only handles integer positions, so we keep an internal
//! `scale = 10^digits` factor and store the value as the integer
//! `value * scale` (clamped to the Win32 16-bit range). On each
//! notification we read the integer position back, divide by `scale`,
//! and write the formatted value into the buddy.
//!
//! # Example
//! ```no_run
//! use ru_wx::prelude::*;
//!
//! let frame = Frame::builder().with_title("App").with_size(100, 100).build();
//! // Range [-1.0, 1.0], initial 0.5, step 0.1, 2 decimal digits.
//! let sc = SpinCtrlDouble::new(&frame, 0.5, -1.0, 1.0, 0.1, 2);
//! assert!((sc.get_value() - 0.5).abs() < 1e-9);
//! ```

use std::cell::RefCell;
use std::rc::Rc;

use crate::window::frame::Frame;
use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};

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
const UDM_SETPOS: u32 = 0x0467;
#[cfg(target_os = "windows")]
const UDM_GETPOS: u32 = 0x0468;
#[cfg(target_os = "windows")]
const UDM_SETBUDDY: u32 = 0x0469;

/// `UDS_ALIGNRIGHT` — the up-down control is placed next to the
/// right edge of its buddy.
#[cfg(target_os = "windows")]
const UDS_ALIGNRIGHT: u32 = 0x0004;
/// `UDS_SETBUDDYINT` — the up-down control writes the integer
/// position into the buddy text. We use this and then *overwrite* the
/// buddy text with the formatted double, so the user briefly sees an
/// integer between notifications — acceptable for short click bursts.
#[cfg(target_os = "windows")]
const UDS_SETBUDDYINT: u32 = 0x0002;
/// `UDS_ARROWKEYS` — the up-down control processes the up / down
/// arrow keys.
#[cfg(target_os = "windows")]
const UDS_ARROWKEYS: u32 = 0x0020;
/// `UDS_HOTTRACK` — hot-track the arrows as the user drags.
#[cfg(target_os = "windows")]
const UDS_HOTTRACK: u32 = 0x0008;
/// `UDS_NOTHOUSANDS` — no thousands separator in the buddy text.
#[cfg(target_os = "windows")]
const UDS_NOTHOUSANDS: u32 = 0x0010;

// ── Inner type ───────────────────────────────────────────────────────

struct SpinCtrlDoubleInner {
    #[cfg(target_os = "windows")]
    updown_hwnd: HWND,
    #[cfg(target_os = "windows")]
    edit_hwnd: HWND,
    id: u16,
    rect: Rect,
    /// Min in user units (the value the user sees).
    min: f64,
    /// Max in user units.
    max: f64,
    /// Current value in user units.
    value: f64,
    /// Step in user units.
    increment: f64,
    /// Number of digits shown after the decimal point.
    digits: u32,
    enabled: bool,
    visible: bool,
}

#[derive(Clone)]
pub struct SpinCtrlDouble {
    inner: Rc<RefCell<SpinCtrlDoubleInner>>,
}

impl SpinCtrlDouble {
    /// Create a new floating-point spin control.
    ///
    /// * `initial` — starting value, clamped to `[min, max]`.
    /// * `min`, `max` — value range in user units.
    /// * `increment` — step in user units (must be > 0).
    /// * `digits` — number of decimal places shown.
    pub fn new<W: Window>(
        parent_in: &W,
        initial: f64,
        min: f64,
        max: f64,
        increment: f64,
        digits: u32,
    ) -> Self {
        let id = next_control_id();
        let increment = if increment > 0.0 { increment } else { 1.0 };
        let initial = initial.max(min).min(max);

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let (updown_hwnd, edit_hwnd) = unsafe {
            let parent = parent_in.hwnd();

            let wide_edit = to_wide("EDIT");
            let edit_hwnd = CreateWindowExW(
                WS_EX_CLIENTEDGE,
                wide_edit.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | (ES_AUTOHSCROLL as u32) | (ES_NUMBER as u32),
                0,
                0,
                100,
                24,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );

            let wide_class = to_wide("msctls_updown32");
            let updown_hwnd = CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD
                    | WS_VISIBLE
                    | UDS_ALIGNRIGHT
                    | UDS_SETBUDDYINT
                    | UDS_ARROWKEYS
                    | UDS_HOTTRACK
                    | UDS_NOTHOUSANDS,
                0,
                0,
                100,
                24,
                parent,
                (id as usize + 1) as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );

            SendMessageW(updown_hwnd, UDM_SETBUDDY, edit_hwnd as usize, 0);
            SendMessageW(updown_hwnd, 0x046D /* UDM_SETBASE */, 10, 0);

            (updown_hwnd, edit_hwnd)
        };

        #[cfg(not(target_os = "windows"))]
        let _ = parent_in;

        let s = SpinCtrlDouble {
            inner: Rc::new(RefCell::new(SpinCtrlDoubleInner {
                #[cfg(target_os = "windows")]
                updown_hwnd,
                #[cfg(target_os = "windows")]
                edit_hwnd,
                id,
                rect: Rect::new(0, 0, 100, 24),
                min,
                max,
                value: initial,
                increment,
                digits,
                enabled: true,
                visible: true,
            })),
        };

        s.set_range(min, max);
        s.set_value(initial);

        s
    }

    /// Set the value range.
    pub fn set_range(&self, min: f64, max: f64) {
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
            // Pack the integer range for the up-down control: positions
            // are 0..=range (range = max - min in 1/10^digits units).
            let (imin, imax) = self.int_range();
            let range = ((imax as u32) << 16) | (imin as u32);
            SendMessageW(
                self.inner.borrow().updown_hwnd,
                UDM_SETRANGE,
                0,
                range as isize,
            );
        }
    }

    /// Set the current value. Clamped to `[min, max]`.
    pub fn set_value(&self, value: f64) {
        let v = {
            let mut inner = self.inner.borrow_mut();
            let v = value.max(inner.min).min(inner.max);
            inner.value = v;
            v
        };
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let ipos = self.value_to_int(v);
            SendMessageW(
                self.inner.borrow().updown_hwnd,
                UDM_SETPOS,
                0,
                ipos as isize,
            );

            // Overwrite the buddy text with the formatted double.
            let text = format!("{:.*}", self.inner.borrow().digits as usize, v);
            let wide = to_wide(&text);
            SetWindowTextW(self.inner.borrow().edit_hwnd, wide.as_ptr());
        }
    }

    /// Return the current value.
    pub fn get_value(&self) -> f64 {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let ipos =
                unsafe { SendMessageW(self.inner.borrow().updown_hwnd, UDM_GETPOS, 0, 0) } as i32;
            let v = self.int_to_value(ipos);
            self.inner.borrow_mut().value = v;
            v
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.inner.borrow().value
        }
    }

    /// Return the current minimum.
    pub fn get_min(&self) -> f64 {
        self.inner.borrow().min
    }

    /// Return the current maximum.
    pub fn get_max(&self) -> f64 {
        self.inner.borrow().max
    }

    /// Return the current range as `(min, max)`.
    pub fn get_range(&self) -> (f64, f64) {
        (self.get_min(), self.get_max())
    }

    /// Return the current step (increment).
    pub fn get_increment(&self) -> f64 {
        self.inner.borrow().increment
    }

    /// Set the step (increment). The value must be positive.
    pub fn set_increment(&self, increment: f64) {
        let inc = if increment > 0.0 { increment } else { 1.0 };
        self.inner.borrow_mut().increment = inc;
    }

    /// Return the number of decimal digits shown.
    pub fn get_digits(&self) -> u32 {
        self.inner.borrow().digits
    }

    /// Set the number of decimal digits shown. The displayed value
    /// is reformatted on the next change.
    pub fn set_digits(&self, digits: u32) {
        self.inner.borrow_mut().digits = digits;
        self.set_value(self.get_value());
    }

    /// Register a callback fired when the value changes. The new
    /// value (in user units) is passed to the callback.
    pub fn on_value_change<F: FnMut(f64) + 'static>(&self, frame: &Frame, mut callback: F) {
        let id = self.inner.borrow().id;
        let inner = self.inner.clone();
        frame.register_command_handler(
            id,
            Box::new(move || {
                #[cfg(target_os = "windows")]
                {
                    let hwnd = inner.borrow().updown_hwnd;
                    // SAFETY: FFI call to SendMessageW; `hwnd` is a live window and `msg` / `wParam` / `lParam` are valid for that window.
                    let ipos = unsafe { SendMessageW(hwnd, UDM_GETPOS, 0, 0) } as i32;
                    let v = inner.borrow().min + (ipos as f64) * inner.borrow().increment;
                    let v = v.max(inner.borrow().min).min(inner.borrow().max);
                    inner.borrow_mut().value = v;
                    callback(v);
                }
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = inner;
                    callback(0.0);
                }
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

    // ── internal helpers ──────────────────────────────────────────

    /// Convert a user-unit value to the integer position used by the
    /// Win32 up-down control.
    #[cfg(target_os = "windows")]
    fn value_to_int(&self, v: f64) -> i32 {
        let inner = self.inner.borrow();
        let scale = 10f64.powi(inner.digits as i32);
        let step_int = (inner.increment * scale).round().max(1.0) as i32;
        let pos = ((v - inner.min) * scale / step_int as f64).round() as i32;
        pos.clamp(i16::MIN as i32, i16::MAX as i32)
    }

    /// Convert an integer position back to a user-unit value.
    #[cfg(target_os = "windows")]
    fn int_to_value(&self, ipos: i32) -> f64 {
        let inner = self.inner.borrow();
        let scale = 10f64.powi(inner.digits as i32);
        let step_int = (inner.increment * scale).round().max(1.0) as i32;
        let v = inner.min + (ipos as f64) * step_int as f64 / scale;
        v.max(inner.min).min(inner.max)
    }

    /// The integer position range to send to `UDM_SETRANGE`.
    #[cfg(target_os = "windows")]
    fn int_range(&self) -> (i32, i32) {
        let inner = self.inner.borrow();
        let scale = 10f64.powi(inner.digits as i32);
        let step_int = (inner.increment * scale).round().max(1.0) as i32;
        let n = (((inner.max - inner.min) * scale) / step_int as f64).round() as i32;
        (0, n.clamp(0, i16::MAX as i32))
    }
}

impl Widget for SpinCtrlDoubleInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.edit_hwnd as isize
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
                self.edit_hwnd,
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
            MoveWindow(
                self.edit_hwnd,
                self.rect.x,
                self.rect.y,
                w as i32,
                h as i32,
                1,
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
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            ShowWindow(self.edit_hwnd, if visible { SW_SHOW } else { SW_HIDE });
            ShowWindow(self.updown_hwnd, if visible { SW_SHOW } else { SW_HIDE });
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
            EnableWindow(self.edit_hwnd, if enabled { 1 } else { 0 });
            EnableWindow(self.updown_hwnd, if enabled { 1 } else { 0 });
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// The Win32 constants we use must match `<commctrl.h>`.
    #[cfg(target_os = "windows")]
    #[test]
    fn win32_constants_pinned() {
        assert_eq!(UDM_SETRANGE, 0x0465);
        assert_eq!(UDM_SETPOS, 0x0467);
        assert_eq!(UDM_GETPOS, 0x0468);
        assert_eq!(UDM_SETBUDDY, 0x0469);
        assert_eq!(UDS_ALIGNRIGHT, 0x0004);
        assert_eq!(UDS_SETBUDDYINT, 0x0002);
        assert_eq!(UDS_ARROWKEYS, 0x0020);
        assert_eq!(UDS_HOTTRACK, 0x0008);
        assert_eq!(UDS_NOTHOUSANDS, 0x0010);
    }
}
