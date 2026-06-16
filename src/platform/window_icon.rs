//! Default top-level window icon for `ru_wx` applications.
//!
//! Every [`crate::Frame`](crate::window::frame::Frame), [`crate::Dialog`](crate::window::dialog::Dialog),
//! and other top-level window created by the library receives the
//! embedded `ru_wx` gear logo unless the caller sets a custom icon
//! (e.g. [`crate::Frame::set_icon`](crate::window::frame::Frame::set_icon)).

#[cfg(target_os = "windows")]
use std::sync::Once;

#[cfg(target_os = "windows")]
use crate::dc::icon::svg_bytes_to_hicon;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::InvalidateRect;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    LoadIconW, SendMessageW, HICON, ICON_BIG, ICON_SMALL, IDI_APPLICATION, WM_SETICON,
};

/// Embedded SVG (no web fonts) — rasterised at runtime via `resvg`.
const ICON_SVG: &[u8] = include_bytes!("../../assets/ru_wx_window_icon.svg");

/// Large icon for the title bar / Alt+Tab (32×32 logical pixels).
#[cfg(target_os = "windows")]
const ICON_BIG_PX: u32 = 32;

/// Small icon for the task bar (16×16 logical pixels).
#[cfg(target_os = "windows")]
const ICON_SMALL_PX: u32 = 16;

#[cfg(target_os = "windows")]
fn system_fallback() -> HICON {
    // SAFETY: `IDI_APPLICATION` is a standard Win32 stock icon.
    unsafe { LoadIconW(std::ptr::null_mut(), IDI_APPLICATION) }
}

#[cfg(target_os = "windows")]
fn default_big() -> HICON {
    static mut ICON: HICON = 0 as HICON;
    static ONCE: Once = Once::new();
    unsafe {
        ONCE.call_once(|| {
            ICON = svg_bytes_to_hicon(ICON_SVG, ICON_BIG_PX).unwrap_or_else(system_fallback);
        });
        ICON
    }
}

#[cfg(target_os = "windows")]
fn default_small() -> HICON {
    static mut ICON: HICON = 0 as HICON;
    static ONCE: Once = Once::new();
    unsafe {
        ONCE.call_once(|| {
            ICON = svg_bytes_to_hicon(ICON_SVG, ICON_SMALL_PX).unwrap_or_else(system_fallback);
        });
        ICON
    }
}

/// `HICON` pair for [`WNDCLASSEXW`](windows_sys::Win32::UI::WindowsAndMessaging::WNDCLASSEXW)
/// registration (`hIcon` / `hIconSm`). Handles are cached for the process lifetime.
#[cfg(target_os = "windows")]
pub fn class_icons() -> (HICON, HICON) {
    (default_big(), default_small())
}

/// Apply the library default icon, or `custom` when provided.
///
/// `custom` is used for both large and small slots (typical when the
/// caller supplies a multi-size `.ico` resource). Pass `None` for the
/// built-in `ru_wx` logo.
#[cfg(target_os = "windows")]
pub fn apply_to_hwnd(hwnd: HWND, custom: Option<HICON>) {
    if hwnd.is_null() {
        return;
    }
    let (big, small) = match custom {
        Some(h) if !h.is_null() => (h, h),
        _ => (default_big(), default_small()),
    };
    // SAFETY: `hwnd` is a live top-level window; `WM_SETICON` with
    // `ICON_BIG` / `ICON_SMALL` is the documented Win32 API.
    unsafe {
        SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, big as isize);
        SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, small as isize);
        InvalidateRect(hwnd, std::ptr::null(), 1);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn apply_to_hwnd(_hwnd: isize, _custom: Option<isize>) {}

#[cfg(test)]
mod tests {
    #[test]
    fn default_icon_svg_is_non_empty() {
        assert!(super::ICON_SVG.starts_with(b"<svg"));
        assert!(super::ICON_SVG.len() > 100);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn default_icons_rasterise() {
        let (big, small) = super::class_icons();
        assert!(!big.is_null());
        assert!(!small.is_null());
    }
}
