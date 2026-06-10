//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Progress dialog (`wxProgressDialog`).
//!
//! On Windows this is a modeless top-level window containing:
//!
//! * a static text label showing the current operation message,
//! * a progress bar (`msctls_progress32`) showing the determinate progress,
//! * an optional "Cancel" button.
//!
//! The dialog is *not* modal-blocking. The host application drives it
//! by calling [`ProgressDialog::update`] periodically; that call:
//!
//! 1. Pushes the new value into the gauge and the new text into the
//!    label, then
//! 2. Pumps any pending Win32 messages (so the Cancel button stays
//!    clickable and the window repaints), then
//! 3. Returns `true` if the user has clicked Cancel since the last
//!    call.
//!
//! This mirrors the documented `wxProgressDialog::Update` contract
//! from wxWidgets and is the only safe way to combine a long-running
//! operation on the main thread with a responsive UI: the *caller* is
//! expected to yield back to the dialog by calling `update`.
//!
//! # Example
//! ```no_run
//! use ru_wx::prelude::*;
//!
//! let mut dlg = ProgressDialog::new("Working", "Initialising...", 100);
//! dlg.show();
//! for i in 1..=100 {
//!     if dlg.update(i, &format!("Step {i}/100")) {
//!         // user cancelled
//!         break;
//!     }
//!     // ...do work...
//! }
//! dlg.close();
//! ```

use std::cell::RefCell;
use std::rc::Rc;

#[cfg(target_os = "windows")]
use crate::platform::win32::{next_control_id, to_wide};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{GetStockObject, UpdateWindow, DEFAULT_GUI_FONT};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::SystemServices::SS_LEFT;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Controls::PBS_SMOOTH;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── Win32 constants used by the progress dialog ───────────────────────

/// `IDCANCEL` — standard Win32 id for the "Cancel" button.
#[cfg(target_os = "windows")]
const IDCANCEL: i32 = 2;
/// Class name registered for the progress-dialog top-level window.
#[cfg(target_os = "windows")]
const PROGRESS_CLASS_NAME: &str = "RuWxProgressDialogClass";
/// `PBM_SETPOS` — push the current position into the gauge.
#[cfg(target_os = "windows")]
const PBM_SETPOS: u32 = 0x0402;
/// `PBM_SETRANGE32` — set the 32-bit range.
#[cfg(target_os = "windows")]
const PBM_SETRANGE32: u32 = 0x0406;
/// `PBM_GETPOS` — read the current position back.
#[cfg(target_os = "windows")]
const PBM_GETPOS: u32 = 0x0408;

/// Dialog inner width in pixels.
const DLG_W: i32 = 440;
/// Dialog inner height in pixels (no cancel button).
const DLG_H_NO_CANCEL: i32 = 100;
/// Dialog inner height in pixels (with cancel button).
const DLG_H_WITH_CANCEL: i32 = 150;
/// Padding between controls.
const PAD: i32 = 10;
/// Default label height.
const LABEL_HEIGHT: i32 = 20;
/// Default gauge height.
const GAUGE_HEIGHT: i32 = 22;
/// Default button width.
const BUTTON_WIDTH: i32 = 90;
/// Default button height.
const BUTTON_HEIGHT: i32 = 28;

// ── Inner type ────────────────────────────────────────────────────────

struct ProgressDialogInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(target_os = "windows")]
    hwnd_label: HWND,
    #[cfg(target_os = "windows")]
    hwnd_gauge: HWND,
    #[cfg(target_os = "windows")]
    hwnd_cancel: HWND,
    cancelled: bool,
    closed: bool,
    range: i32,
    value: i32,
    title: String,
    message: String,
    has_cancel: bool,
    /// Marker so non-Windows builds still compile.
    #[cfg(not(target_os = "windows"))]
    _unsupported: (),
}

#[derive(Clone)]
pub struct ProgressDialog {
    inner: Rc<RefCell<ProgressDialogInner>>,
}

// ── Window class registration ────────────────────────────────────────

/// Register the progress-dialog window class (idempotent).
#[cfg(target_os = "windows")]
fn register_progress_class() {
    // SAFETY: Win32 FFI call with validated arguments.
    unsafe {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_name = to_wide(PROGRESS_CLASS_NAME);

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(progress_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            // NULL_BRUSH (5) — we don't want a default background
            // because the parent already paints the dialog frame.
            hbrBackground: GetStockObject(0),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wc);
    }
}

// ── Construction / public API ────────────────────────────────────────

impl ProgressDialog {
    /// Build a new progress dialog with a determinate range.
    ///
    /// The dialog has *no* Cancel button. Use
    /// [`ProgressDialog::with_cancel_button`] to add one.
    pub fn new(title: &str, message: &str, range: i32) -> Self {
        Self::new_internal(title, message, range, false)
    }

    /// Build a new progress dialog that lets the user cancel.
    pub fn with_cancel_button(title: &str, message: &str, range: i32) -> Self {
        Self::new_internal(title, message, range, true)
    }

    fn new_internal(title: &str, message: &str, range: i32, has_cancel: bool) -> Self {
        #[cfg(target_os = "windows")]
        {
            register_progress_class();

            // SAFETY: Win32 FFI calls with validated arguments.
            unsafe {
                let wide_class = to_wide(PROGRESS_CLASS_NAME);
                let wide_title = to_wide(title);
                let hinstance = GetModuleHandleW(std::ptr::null());
                let h = if has_cancel { DLG_H_WITH_CANCEL } else { DLG_H_NO_CANCEL };
                let hwnd = CreateWindowExW(
                    WS_EX_DLGMODALFRAME,
                    wide_class.as_ptr(),
                    wide_title.as_ptr(),
                    WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    DLG_W,
                    h,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    hinstance,
                    std::ptr::null_mut(),
                );

                // Label.
                let wide_msg = to_wide(message);
                let hwnd_label = CreateWindowExW(
                    0,
                    to_wide("STATIC").as_ptr(),
                    wide_msg.as_ptr(),
                    WS_CHILD | WS_VISIBLE | SS_LEFT,
                    PAD,
                    PAD,
                    DLG_W - 2 * PAD,
                    LABEL_HEIGHT,
                    hwnd,
                    std::ptr::null_mut(),
                    hinstance,
                    std::ptr::null_mut(),
                );

                // Gauge.
                let gauge_y = PAD + LABEL_HEIGHT + 6;
                let hwnd_gauge = CreateWindowExW(
                    0,
                    to_wide("msctls_progress32").as_ptr(),
                    std::ptr::null(),
                    WS_CHILD | WS_VISIBLE | PBS_SMOOTH,
                    PAD,
                    gauge_y,
                    DLG_W - 2 * PAD,
                    GAUGE_HEIGHT,
                    hwnd,
                    next_control_id() as usize as HMENU,
                    hinstance,
                    std::ptr::null_mut(),
                );
                // Set the range.
                SendMessageW(hwnd_gauge, PBM_SETRANGE32, 0, range as isize);

                // Cancel button (only if requested).
                let hwnd_cancel = if has_cancel {
                    let wide_cancel = to_wide("Cancel");
                    let button_y = gauge_y + GAUGE_HEIGHT + 14;
                    CreateWindowExW(
                        0,
                        to_wide("BUTTON").as_ptr(),
                        wide_cancel.as_ptr(),
                        WS_CHILD | WS_VISIBLE | (BS_PUSHBUTTON as u32),
                        DLG_W - PAD - BUTTON_WIDTH,
                        button_y,
                        BUTTON_WIDTH,
                        BUTTON_HEIGHT,
                        hwnd,
                        IDCANCEL as usize as HMENU,
                        hinstance,
                        std::ptr::null_mut(),
                    )
                } else {
                    std::ptr::null_mut()
                };

                // Apply a sensible default GUI font to the label and
                // the Cancel button so they look native.
                let hfont = GetStockObject(DEFAULT_GUI_FONT);
                SendMessageW(hwnd_label, WM_SETFONT, hfont as usize, 1);
                if has_cancel {
                    SendMessageW(hwnd_cancel, WM_SETFONT, hfont as usize, 1);
                }

                let dlg = ProgressDialog {
                    inner: Rc::new(RefCell::new(ProgressDialogInner {
                        hwnd,
                        hwnd_label,
                        hwnd_gauge,
                        hwnd_cancel,
                        cancelled: false,
                        closed: false,
                        range,
                        value: 0,
                        title: title.to_string(),
                        message: message.to_string(),
                        has_cancel,
                    })),
                };

                // Park the Rc pointer in GWLP_USERDATA so the WndProc
                // can reach the inner state.
                let raw = Rc::into_raw(dlg.inner.clone());
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw as isize);

                dlg
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            // Non-Windows stub. The type still exists so user code
            // compiles cross-platform.
            let _ = (title, message, range, has_cancel);
            ProgressDialog {
                inner: Rc::new(RefCell::new(ProgressDialogInner {
                    cancelled: false,
                    closed: false,
                    range,
                    value: 0,
                    title: title.to_string(),
                    message: message.to_string(),
                    has_cancel,
                    _unsupported: (),
                })),
            }
        }
    }

    /// Show the dialog (it is created hidden by default).
    pub fn show(&self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments.
        unsafe {
            ShowWindow(self.inner.borrow().hwnd, SW_SHOW);
            UpdateWindow(self.inner.borrow().hwnd);
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = self;
        }
    }

    /// Close and destroy the dialog.
    pub fn close(&mut self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments.
        unsafe {
            if !self.inner.borrow().closed {
                // DestroyWindow synchronously triggers WM_DESTROY,
                // which clears GWLP_USERDATA and consumes the Rc.
                DestroyWindow(self.inner.borrow().hwnd);
            }
        }
        self.inner.borrow_mut().closed = true;
    }

    /// Update the progress value (clamped to `[0, range]`) and the
    /// message label, then pump pending Win32 messages. Returns
    /// `true` if the user has clicked Cancel since the last call.
    pub fn update(&mut self, value: i32, message: &str) -> bool {
        self.update_message(message);
        self.update_value(value)
    }

    /// Update the progress value only; the message label is left
    /// unchanged. Returns `true` if the user has clicked Cancel since
    /// the last call.
    pub fn update_value(&mut self, value: i32) -> bool {
        let clamped = {
            let mut inner = self.inner.borrow_mut();
            let range = inner.range;
            let v = value.max(0).min(range);
            inner.value = v;
            v
        };
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd_gauge, PBM_SETPOS, clamped as usize, 0);
        }
        self.pump();
        self.is_cancelled()
    }

    /// Update the message label only; the progress value is left
    /// unchanged. Returns `true` if the user has clicked Cancel since
    /// the last call.
    pub fn update_message(&mut self, message: &str) -> bool {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments.
        unsafe {
            let wide = to_wide(message);
            SetWindowTextW(self.inner.borrow().hwnd_label, wide.as_ptr());
        }
        self.inner.borrow_mut().message = message.to_string();
        self.pump();
        self.is_cancelled()
    }

    /// Return whether the user has clicked Cancel.
    pub fn is_cancelled(&self) -> bool {
        self.inner.borrow().cancelled
    }

    /// Return whether the dialog has been closed (and the underlying
    /// HWND is gone).
    pub fn is_closed(&self) -> bool {
        self.inner.borrow().closed
    }

    /// Read the current title.
    pub fn title(&self) -> String {
        self.inner.borrow().title.clone()
    }

    /// Read the current message.
    pub fn message(&self) -> String {
        self.inner.borrow().message.clone()
    }

    /// Read the determinate range.
    pub fn range(&self) -> i32 {
        self.inner.borrow().range
    }

    /// Read the current value.
    pub fn value(&self) -> i32 {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: Win32 FFI call with validated arguments.
            let v = unsafe { SendMessageW(self.inner.borrow().hwnd_gauge, PBM_GETPOS, 0, 0) };
            self.inner.borrow_mut().value = v as i32;
        }
        self.inner.borrow().value
    }

    /// Read whether the dialog has a Cancel button.
    pub fn has_cancel_button(&self) -> bool {
        self.inner.borrow().has_cancel
    }

    // ── Internal helpers ───────────────────────────────────────────

    /// Pump pending Win32 messages. This is what makes the dialog
    /// interactive: without it, the Cancel button would never
    /// dispatch its `WM_COMMAND`.
    fn pump(&self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI calls with validated arguments.
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            // Drain the queue. PM_REMOVE ensures we process every
            // message currently pending; if a new one arrives during
            // dispatch we re-enter the loop on the next `update` call.
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    // Re-post so the main message loop can honour it.
                    PostQuitMessage(msg.wParam as i32);
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = self;
        }
    }
}

// ── Window trait (so the dialog itself can act as a parent if needed) ─

#[cfg(target_os = "windows")]
impl crate::core::widget::Window for ProgressDialog {
    fn hwnd(&self) -> HWND {
        self.inner.borrow().hwnd
    }
}

// ── Window procedure ─────────────────────────────────────────────────

/// Window procedure for the progress dialog class.
///
/// * `WM_COMMAND` from the Cancel button (`IDCANCEL`) — flips
///   `cancelled = true` on the inner state.
/// * `WM_CLOSE` — destroys the window.
/// * `WM_DESTROY` — releases the `Rc<RefCell<Inner>>` parked in
///   `GWLP_USERDATA` so it can be freed.
#[cfg(target_os = "windows")]
unsafe extern "system" fn progress_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as i32;
            if id == IDCANCEL {
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
                if ptr != 0 {
                    // We got the raw pointer; rebuild an Rc so we can
                    // mutate the inner state, then re-forget it so the
                    // strong count remains balanced.
                    let rc = Rc::from_raw(ptr as *const RefCell<ProgressDialogInner>);
                    rc.borrow_mut().cancelled = true;
                    let _ = Rc::into_raw(rc);
                }
            }
            0
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let _ = Rc::from_raw(ptr as *const RefCell<ProgressDialogInner>);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// ── Drop ─────────────────────────────────────────────────────────────

impl Drop for ProgressDialog {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments.
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                // WM_DESTROY will already have done this if the user
                // called `close()`. If the user just dropped the
                // dialog, do it here so the GWLP_USERDATA doesn't
                // dangle.
                let _ = Rc::from_raw(ptr as *const RefCell<ProgressDialogInner>);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                DestroyWindow(hwnd);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_dialog_construction() {
        // Non-modal roundtrip test: build the dialog, exercise
        // setters, confirm we never panic. The actual `show` /
        // `update` requires a real windowed message pump and is
        // covered by the windowed smoke test in
        // `examples/showcase_all.rs`.
        let mut dlg = ProgressDialog::new("Working", "Initialising...", 100);
        assert_eq!(dlg.title(), "Working");
        assert_eq!(dlg.message(), "Initialising...");
        assert_eq!(dlg.range(), 100);
        assert_eq!(dlg.value(), 0);
        assert!(!dlg.has_cancel_button());
        assert!(!dlg.is_cancelled());
        assert!(!dlg.is_closed());

        // `update_message` doesn't change the value.
        dlg.update_message("Step 5");
        assert_eq!(dlg.message(), "Step 5");
        assert_eq!(dlg.value(), 0);

        // `update_value` clamps to [0, range].
        dlg.update_value(10);
        assert_eq!(dlg.value(), 10);
        dlg.update_value(500); // over-range
        assert_eq!(dlg.value(), 100);
        dlg.update_value(-3); // under-range
        assert_eq!(dlg.value(), 0);
    }

    #[test]
    fn progress_dialog_with_cancel() {
        let dlg = ProgressDialog::with_cancel_button("Working", "msg", 50);
        assert!(dlg.has_cancel_button());
        assert!(!dlg.is_cancelled());
    }

    #[test]
    fn cancelled_flag_starts_false() {
        let dlg = ProgressDialog::new("t", "m", 10);
        assert!(!dlg.is_cancelled());
    }

    #[test]
    fn closed_flag_starts_false() {
        let dlg = ProgressDialog::new("t", "m", 10);
        assert!(!dlg.is_closed());
    }
}
