//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Win32 platform utilities and helpers

#[cfg(target_os = "windows")]
use crate::platform::ids;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::GetDeviceCaps;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::LOGPIXELSX;

/// Convert a Rust &str to a null-terminated wide string (UTF-16)
pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Query the horizontal DPI of a device context.
///
/// Returns the `LOGPIXELSX` value, i.e. the number of pixels per
/// logical inch in the horizontal direction. Defaults to 96 (the
/// standard DPI) when the HDC is null or the call fails.
#[cfg(target_os = "windows")]
#[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper around GetDeviceCaps
pub fn get_device_caps_dpi(hdc: windows_sys::Win32::Graphics::Gdi::HDC) -> u32 {
    if hdc.is_null() {
        return 96;
    }
    // SAFETY: `hdc` is non-null (checked above) and was returned by
    // `GetDC` / `CreateCompatibleDC` / similar; `LOGPIXELSX` is a valid
    // capability index.
    // SAFETY: FFI call to GetDeviceCaps on a live GDI handle returned by the matching Create/Get call.
    // We coerce the capability index with `unwrap_or(0)` so an
    // unexpected widening failure falls back to the standard 96-DPI
    // default rather than panicking inside a UI code path.
    let dpi = unsafe { GetDeviceCaps(hdc, LOGPIXELSX.try_into().unwrap_or(0)) };
    if dpi > 0 {
        dpi as u32
    } else {
        96u32
    }
}

/// Get a unique control ID for Win32 child windows.
pub fn next_control_id() -> u16 {
    ids::next_control_id()
}

/// Read the current window text via `GetWindowTextW`.
///
/// Returns an empty string when Win32 reports an error (`len <= 0`)
/// so a `-1` length cannot be cast to `usize::MAX` and abort the
/// process.
#[cfg(target_os = "windows")]
#[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper; `hwnd` must be live
pub fn read_window_text(hwnd: windows_sys::Win32::Foundation::HWND) -> String {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowTextLengthW, GetWindowTextW};

    // SAFETY: `hwnd` must be a live window handle; callers validate
    // this before calling.
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return String::new();
    }
    let buf_len = (len as usize).saturating_add(1);
    let mut buf = vec![0u16; buf_len];
    // SAFETY: buffer is `len + 1` wide chars; `GetWindowTextW` copies
    // at most `len` characters plus a terminating NUL.
    let copied = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1) };
    if copied <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..copied as usize])
}

/// Get a unique menu item ID
pub fn next_menu_id() -> u16 {
    ids::next_menu_id()
}

/// Apply per-pixel alpha to a top-level or popup window (`SetLayeredWindowAttributes`).
#[cfg(target_os = "windows")]
#[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper; `hwnd` must be live
pub fn set_window_opacity(hwnd: windows_sys::Win32::Foundation::HWND, opacity: u8) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongW, SetLayeredWindowAttributes, SetWindowLongW, GWL_EXSTYLE, LWA_ALPHA,
        WS_EX_LAYERED,
    };

    if hwnd.is_null() {
        return;
    }
    // SAFETY: Win32 style/opacity calls on a live HWND owned by the caller.
    unsafe {
        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
        SetWindowLongW(hwnd, GWL_EXSTYLE, ex | WS_EX_LAYERED as i32);
        SetLayeredWindowAttributes(hwnd, 0, opacity, LWA_ALPHA);
    }
}

/// Screen-space rect of a window — top-level or child control.
///
/// `GetWindowRect` reports physical pixels with a top-left origin, which
/// matches the normalised convention produced by the macOS backend's
/// `mac_window_frame_rect`. Accepts any live `HWND`, including child
/// controls, so it doubles as the per-widget bounds reader.
///
/// Returns an all-zero rect when the handle is not a valid window.
#[cfg(target_os = "windows")]
#[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper; `hwnd` must be live
pub fn window_rect(hwnd: windows_sys::Win32::Foundation::HWND) -> crate::core::geometry::Rect {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;

    let mut rc: RECT = unsafe { std::mem::zeroed() };
    // SAFETY: `hwnd` is expected to be a live window; if the call fails
    // Win32 leaves `rc` untouched and returns 0, which we handle below.
    if unsafe { GetWindowRect(hwnd, &mut rc) } == 0 {
        return crate::core::geometry::Rect::new(0, 0, 0, 0);
    }
    crate::core::geometry::Rect::new(
        rc.left,
        rc.top,
        (rc.right - rc.left).max(0) as u32,
        (rc.bottom - rc.top).max(0) as u32,
    )
}

/// Client-area size (physical pixels) of a window or control.
///
/// This excludes the window chrome for top-level windows and returns the
/// full control area for child controls. Returns `(0, 0)` on failure.
#[cfg(target_os = "windows")]
#[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper; `hwnd` must be live
pub fn client_size(hwnd: windows_sys::Win32::Foundation::HWND) -> (u32, u32) {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect;

    let mut rc: RECT = unsafe { std::mem::zeroed() };
    // SAFETY: see `window_rect`.
    if unsafe { GetClientRect(hwnd, &mut rc) } == 0 {
        return (0, 0);
    }
    (
        (rc.right - rc.left).max(0) as u32,
        (rc.bottom - rc.top).max(0) as u32,
    )
}
