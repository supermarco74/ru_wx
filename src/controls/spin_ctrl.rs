//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Numeric stepper (`wxSpinCtrl`).
//!
//! On Windows, the spin control is realised with the common control
//! class `msctls_updown32`. The control exposes a value in
//! `[min, max]`, with up/down arrow buttons that increment or
//! decrement the value.
//!
//! To match the `wxSpinCtrl` UX (an editable number next to the
//! arrows), we make the up-down control a *buddy* of an `EDIT` text
//! box that displays the current value. The user can either type a
//! number into the text box or click the up/down arrows to step
//! the value.

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
const UDM_SETRANGE: u32 = 0x0465; // wparam = (max<<16)|min
#[cfg(target_os = "windows")]
#[allow(dead_code)] // Win32 ABI surface — for future get-range helper
const UDM_GETRANGE: u32 = 0x0466;
#[cfg(target_os = "windows")]
const UDM_SETPOS: u32 = 0x0467;
#[cfg(target_os = "windows")]
const UDM_GETPOS: u32 = 0x0468;
#[cfg(target_os = "windows")]
const UDM_SETBUDDY: u32 = 0x0469;
#[cfg(target_os = "windows")]
#[allow(dead_code)] // Win32 ABI surface — for future set-accel helper
const UDM_SETACCEL: u32 = 0x046B;
#[cfg(target_os = "windows")]
const UDM_SETBASE: u32 = 0x046D;
#[cfg(target_os = "windows")]
#[allow(dead_code)] // Win32 ABI surface — for future get-base helper
const UDM_GETBASE: u32 = 0x046E;

/// `UDN_DELTAPOS` — up-down control position is about to change.
#[cfg(target_os = "windows")]
const UDN_DELTAPOS: u32 = 0xFFFF_FD09;

/// `UDS_ALIGNRIGHT` — the up-down control is placed next to the right
/// edge of its buddy.
#[cfg(target_os = "windows")]
const UDS_ALIGNRIGHT: u32 = 0x0004;
/// `UDS_SETBUDDYINT` — the up-down control sets the buddy text to the
/// current integer value.
#[cfg(target_os = "windows")]
const UDS_SETBUDDYINT: u32 = 0x0002;
/// `UDS_ARROWKEYS` — the up-down control processes the up/down arrow
/// keys.
#[cfg(target_os = "windows")]
const UDS_ARROWKEYS: u32 = 0x0020;
/// `UDS_HOTTRACK` — the up-down control highlights the arrows as the
/// user drags.
#[cfg(target_os = "windows")]
const UDS_HOTTRACK: u32 = 0x0008;
/// `UDS_NOTHOUSANDS` — no thousands separator in the buddy text.
#[cfg(target_os = "windows")]
const UDS_NOTHOUSANDS: u32 = 0x0010;

// ── Inner type ───────────────────────────────────────────────────────

struct SpinCtrlInner {
    #[cfg(target_os = "windows")]
    updown_hwnd: HWND,
    #[cfg(target_os = "windows")]
    edit_hwnd: HWND,
    #[cfg(target_os = "windows")]
    updown_id: u16,
    #[cfg(target_os = "windows")]
    notify_frame: Option<Frame>,
    id: u16,
    rect: Rect,
    min: i32,
    max: i32,
    value: i32,
    enabled: bool,
    visible: bool,
}

#[derive(Clone)]
pub struct SpinCtrl {
    inner: Rc<RefCell<SpinCtrlInner>>,
}

impl SpinCtrl {
    /// Create a new spin control with the given value range. The
    /// initial value is clamped to `[min, max]`.
    pub fn new<W: Window>(parent_in: &W, min: i32, max: i32, initial: i32) -> Self {
        let edit_id = next_control_id();
        let updown_id = next_control_id();
        let initial = initial.max(min).min(max);

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let (updown_hwnd, edit_hwnd) = unsafe {
            let parent = parent_in.hwnd();

            // 1. Create the EDIT control (buddy). It will be sized and
            //    positioned by the sizer; here we just give it an
            //    initial size.
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
                edit_id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );

            // 2. Create the up-down control with UDS_ALIGNRIGHT so it
            //    sits next to the buddy.
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
                updown_id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );

            // 3. Make the up-down control target the edit control.
            SendMessageW(updown_hwnd, UDM_SETBUDDY, edit_hwnd as usize, 0);
            // Use base-10 (decimal).
            SendMessageW(updown_hwnd, UDM_SETBASE, 10, 0);

            (updown_hwnd, edit_hwnd)
        };

        #[cfg(not(target_os = "windows"))]
        let _ = parent_in;

        let s = SpinCtrl {
            inner: Rc::new(RefCell::new(SpinCtrlInner {
                #[cfg(target_os = "windows")]
                updown_hwnd,
                #[cfg(target_os = "windows")]
                edit_hwnd,
                #[cfg(target_os = "windows")]
                updown_id,
                #[cfg(target_os = "windows")]
                notify_frame: None,
                id: edit_id,
                rect: Rect::new(0, 0, 100, 24),
                min,
                max,
                value: initial,
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
            // UDM_SETRANGE packs max and min into lparam: (max << 16) | min
            // Win32 max range is 16-bit; clamp to fit.
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
                self.inner.borrow().updown_hwnd,
                UDM_SETRANGE,
                0,
                range as isize,
            );
        }
    }

    /// Set the current value.
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
            // UDM_SETPOS: the up-down control will (because of
            // UDS_SETBUDDYINT) write the new value to the buddy text.
            SendMessageW(self.inner.borrow().updown_hwnd, UDM_SETPOS, 0, v as isize);
        }
    }

    /// Return the current value.
    pub fn get_value(&self) -> i32 {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let v =
                unsafe { SendMessageW(self.inner.borrow().updown_hwnd, UDM_GETPOS, 0, 0) } as i32;
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

    /// Register a callback fired when the value changes.
    pub fn on_value_change<F: FnMut() + 'static>(&self, frame: &Frame, callback: F) {
        let edit_id = self.inner.borrow().id;
        let inner = self.inner.clone();
        #[cfg(target_os = "windows")]
        {
            let updown_id = self.inner.borrow().updown_id;
            self.inner.borrow_mut().notify_frame = Some(frame.clone());
            let callback = Rc::new(RefCell::new(callback));
            let inner_for_edit = inner.clone();
            let cb_edit = callback.clone();
            frame.register_command_handler(
                edit_id,
                Box::new(move || {
                    let hwnd = inner_for_edit.borrow().updown_hwnd;
                    // SAFETY: FFI call to SendMessageW; `hwnd` is a live up-down control.
                    let v = unsafe { SendMessageW(hwnd, UDM_GETPOS, 0, 0) } as i32;
                    inner_for_edit.borrow_mut().value = v;
                    cb_edit.borrow_mut()();
                }),
            );
            let inner_for_udn = inner.clone();
            let cb_udn = callback.clone();
            frame.register_notify_handler(
                updown_id,
                Box::new(move |code| {
                    if code != UDN_DELTAPOS {
                        return;
                    }
                    let hwnd = inner_for_udn.borrow().updown_hwnd;
                    // SAFETY: FFI call to SendMessageW; `hwnd` is a live up-down control.
                    let v = unsafe { SendMessageW(hwnd, UDM_GETPOS, 0, 0) } as i32;
                    inner_for_udn.borrow_mut().value = v;
                    cb_udn.borrow_mut()();
                }),
            );
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (frame, callback);
        }
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

impl Widget for SpinCtrlInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            // Return the buddy edit's handle — the sizer positions
            // both as a single unit and `MoveWindow` is called on the
            // edit, so reporting the edit is correct.
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
            // The up-down control is UDS_ALIGNRIGHT, so it follows the
            // buddy automatically. We only need to move the buddy.
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
            // Move the buddy; the up-down control is positioned
            // automatically by UDS_ALIGNRIGHT to the right of the
            // buddy.
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
