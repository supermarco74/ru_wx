//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! "Busy info" floating window (`wxBusyInfo`).
//!
//! `wxBusyInfo` is a non-interactive top-level window with a centred
//! message. The user is expected to:
//!
//! 1. Create a `BusyInfo` (which constructs and shows the window),
//! 2. Do some long-running work,
//! 3. Drop the `BusyInfo` when done (which destroys the window).
//!
//! The window has no title bar, no system menu, and no focus
//! capability — it is purely a visual cue. Internally we just create
//! a small top-level window with a centred `STATIC` label, then
//! destroy it in `Drop`.
//!
//! # Example
//! ```no_run
//! use ru_wx::prelude::*;
//!
//! let _busy = BusyInfo::new("Loading, please wait...");
//! // ...do work...
//! // _busy goes out of scope, window disappears.
//! ```

use std::cell::RefCell;
use std::rc::Rc;

#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{GetStockObject, UpdateWindow, DEFAULT_GUI_FONT};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::SystemServices::SS_CENTER;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Class name registered for the busy-info window.
#[cfg(target_os = "windows")]
const BUSY_INFO_CLASS_NAME: &str = "RuWxBusyInfoClass";
/// Inner width in pixels.
const W: i32 = 360;
/// Inner height in pixels.
const H: i32 = 80;
/// Padding inside the window.
const PAD: i32 = 12;

struct BusyInfoInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(not(target_os = "windows"))]
    _unsupported: (),
}

#[derive(Clone)]
pub struct BusyInfo {
    inner: Rc<RefCell<BusyInfoInner>>,
}

/// Register the busy-info window class (idempotent).
#[cfg(target_os = "windows")]
fn register_busy_info_class() {
    // SAFETY: Win32 FFI call with validated arguments.
    unsafe {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_name = to_wide(BUSY_INFO_CLASS_NAME);

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(busy_info_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: GetStockObject(5) as _, // NULL_BRUSH
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wc);
    }
}

impl BusyInfo {
    /// Create and show a new busy-info window with the given message.
    pub fn new(message: &str) -> Self {
        #[cfg(target_os = "windows")]
        {
            register_busy_info_class();

            // SAFETY: Win32 FFI calls with validated arguments.
            unsafe {
                let wide_class = to_wide(BUSY_INFO_CLASS_NAME);
                let wide_msg = to_wide(message);
                let hinstance = GetModuleHandleW(std::ptr::null());

                let hwnd = CreateWindowExW(
                    WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
                    wide_class.as_ptr(),
                    std::ptr::null(),
                    WS_POPUP | WS_BORDER,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    W,
                    H,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    hinstance,
                    std::ptr::null_mut(),
                );

                // Centred label.
                let hwnd_label = CreateWindowExW(
                    0,
                    to_wide("STATIC").as_ptr(),
                    wide_msg.as_ptr(),
                    WS_CHILD | WS_VISIBLE | SS_CENTER,
                    PAD,
                    PAD,
                    W - 2 * PAD,
                    H - 2 * PAD,
                    hwnd,
                    std::ptr::null_mut(),
                    hinstance,
                    std::ptr::null_mut(),
                );
                let hfont = GetStockObject(DEFAULT_GUI_FONT);
                SendMessageW(hwnd_label, WM_SETFONT, hfont as usize, 1);

                // Centre on screen.
                centre_window_on_screen(hwnd, W, H);
                ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                UpdateWindow(hwnd);

                let dlg = BusyInfo {
                    inner: Rc::new(RefCell::new(BusyInfoInner { hwnd })),
                };

                let raw = Rc::into_raw(dlg.inner.clone());
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw as isize);

                dlg
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            // Non-Windows stub.
            let _ = message;
            BusyInfo {
                inner: Rc::new(RefCell::new(BusyInfoInner {
                    _unsupported: (),
                })),
            }
        }
    }

    /// Update the message text shown in the window.
    pub fn set_message(&self, message: &str) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments.
        unsafe {
            // Find the only STATIC child of the busy-info window and
            // re-text it. We do this by enumerating children with
            // GetWindow(GW_CHILD) since BusyInfo only has one.
            let child = GetWindow(self.inner.borrow().hwnd, GW_CHILD);
            if !child.is_null() {
                let wide = to_wide(message);
                SetWindowTextW(child, wide.as_ptr());
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = message;
        }
    }
}

// ── Drop ─────────────────────────────────────────────────────────────

impl Drop for BusyInfo {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments.
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            if hwnd.is_null() {
                return;
            }
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let _ = Rc::from_raw(ptr as *const RefCell<BusyInfoInner>);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            DestroyWindow(hwnd);
        }
    }
}

// ── Window procedure ─────────────────────────────────────────────────

#[cfg(target_os = "windows")]
unsafe extern "system" fn busy_info_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let _ = Rc::from_raw(ptr as *const RefCell<BusyInfoInner>);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Centre a window of size `(w, h)` on the primary monitor.
#[cfg(target_os = "windows")]
unsafe fn centre_window_on_screen(hwnd: HWND, w: i32, h: i32) {
    // SAFETY: Win32 FFI call with validated arguments.
    unsafe {
        let rect: RECT = std::mem::zeroed();
        // SM_CXSCREEN / SM_CYSCREEN = primary monitor size.
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let x = (screen_w - w) / 2;
        let y = (screen_h - h) / 2;
        SetWindowPos(
            hwnd,
            HWND_TOP,
            x,
            y,
            w,
            h,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
        let _ = rect;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn busy_info_construction() {
        // The constructor creates and shows a top-level window, so
        // we don't actually invoke it in unit tests. Just verify
        // the type is constructible in a no-op path.
        //
        // The full windowed test lives in
        // `examples/showcase_all.rs`.
        let _ = std::mem::size_of::<BusyInfo>();
        let _ = BUSY_INFO_CLASS_NAME.len();
    }
}
