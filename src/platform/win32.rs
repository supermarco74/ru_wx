//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Win32 platform utilities and helpers

use std::sync::atomic::{AtomicU16, Ordering};

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

/// Global counter for unique control IDs
static NEXT_CONTROL_ID: AtomicU16 = AtomicU16::new(100);

/// Get a unique control ID for Win32 child windows.
///
/// IDs are allocated from 100 upward and must stay below 9000 (the
/// menu-item ID range starts at 9000). Panics if the space is
/// exhausted to avoid silent ID collisions.
pub fn next_control_id() -> u16 {
    let id = NEXT_CONTROL_ID.fetch_add(1, Ordering::Relaxed);
    if id >= 9000 {
        panic!("ru_wx: control ID space exhausted (IDs must stay below 9000)");
    }
    id
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

/// Global counter for unique menu item IDs (starting at 9000 to avoid collision with control IDs)
static NEXT_MENU_ID: AtomicU16 = AtomicU16::new(9000);

/// Get a unique menu item ID
pub fn next_menu_id() -> u16 {
    NEXT_MENU_ID.fetch_add(1, Ordering::Relaxed)
}
