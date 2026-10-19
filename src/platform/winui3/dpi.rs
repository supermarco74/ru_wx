//! Per-Monitor V2 DPI helpers.
//!
//! The build script already embeds a `<dpiAwareness>PerMonitorV2</dpiAwareness>`
//! flag into every WinUI3 binary, so the OS wires the process up
//! at the start. We expose two utility functions:
//!
//! * [`scale`]  — scale a `i32` value from 96-DPI logical pixels to the
//!   current system DPI of a given window.
//! * [`query`]  — query the current DPI of an `HWND` (returns 96 when
//!   the call fails).
//!
//! They are pure wrappers around the documented Win32 API
//! (`GetDpiForWindow`) — cheap, infallible, and safe to call on any
//! thread that owns a valid `HWND`.

#![allow(non_snake_case)]

use windows_sys::Win32::Foundation::HWND;

/// Fallback DPI returned when `GetDpiForWindow` is unavailable
/// (old Windows versions, headless contexts).
pub const DEFAULT_DPI: u32 = 96;

/// Query the system DPI of `hwnd`.
///
/// Returns [`DEFAULT_DPI`] (96) if the call fails or `hwnd` is null.
#[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper; `hwnd` must be live
pub fn query(hwnd: HWND) -> u32 {
    if hwnd.is_null() {
        return DEFAULT_DPI;
    }
    // SAFETY: `GetDpiForWindow` is documented to take any live HWND
    // and return `0` on error. Available since Windows 10 1607.
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    if dpi == 0 {
        DEFAULT_DPI
    } else {
        dpi
    }
}

/// Scale a 96-DPI logical pixel count to the current DPI of `hwnd`.
pub fn scale(hwnd: HWND, value: i32) -> i32 {
    let dpi = query(hwnd);
    ((value as f64) * dpi as f64 / DEFAULT_DPI as f64).round() as i32
}

/// Scale `(width, height)` together. Convenience for
/// `SetWindowPos(width, height)` calls that need HiDPI scaling.
pub fn scale_size(hwnd: HWND, width: i32, height: i32) -> (i32, i32) {
    (scale(hwnd, width), scale(hwnd, height))
}

// Win32 binding for `GetDpiForWindow`. Not re-exported by
// `windows-sys 0.59`; the constant lives in `winuser.h`.
#[link(name = "user32.dll", kind = "raw-dylib", modifiers = "+verbatim")]
unsafe extern "system" {
    fn GetDpiForWindow(hwnd: HWND) -> u32;
}
