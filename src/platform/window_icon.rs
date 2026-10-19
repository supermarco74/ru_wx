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

#[cfg(target_os = "macos")]
use objc2::rc::Retained;
#[cfg(target_os = "macos")]
use objc2::AnyThread;
#[cfg(target_os = "macos")]
use objc2_app_kit::NSImage;
#[cfg(target_os = "macos")]
use objc2_foundation::MainThreadMarker;
#[cfg(target_os = "macos")]
use std::cell::{Cell, RefCell};

use crate::platform::NativeHandle;

/// Embedded SVG (no web fonts) — rasterised at runtime via `resvg`.
const ICON_SVG: &[u8] = include_bytes!("../../assets/ru_wx_window_icon.svg");

/// Large icon for the title bar / Alt+Tab (32×32 logical pixels).
#[cfg(target_os = "windows")]
const ICON_BIG_PX: u32 = 32;

/// Small icon for the task bar (16×16 logical pixels).
#[cfg(target_os = "windows")]
const ICON_SMALL_PX: u32 = 16;

#[cfg(target_os = "macos")]
const ICON_PX: u32 = 32;

#[cfg(target_os = "windows")]
fn system_fallback() -> NativeHandle {
    // SAFETY: `IDI_APPLICATION` is a standard Win32 stock icon.
    let hicon = unsafe { LoadIconW(std::ptr::null_mut(), IDI_APPLICATION) };
    hicon as NativeHandle
}

#[cfg(target_os = "windows")]
fn default_big() -> NativeHandle {
    static mut ICON: NativeHandle = 0;
    static ONCE: Once = Once::new();
    unsafe {
        ONCE.call_once(|| {
            ICON = svg_bytes_to_hicon(ICON_SVG, ICON_BIG_PX).unwrap_or_else(system_fallback);
        });
        ICON
    }
}

#[cfg(target_os = "windows")]
fn default_small() -> NativeHandle {
    static mut ICON: NativeHandle = 0;
    static ONCE: Once = Once::new();
    unsafe {
        ONCE.call_once(|| {
            ICON = svg_bytes_to_hicon(ICON_SVG, ICON_SMALL_PX).unwrap_or_else(system_fallback);
        });
        ICON
    }
}

#[cfg(target_os = "macos")]
thread_local! {
    /// Cached default icon. `default_image` requires a
    /// `MainThreadMarker`, so the cache only ever lives on the main
    /// thread and a thread-local avoids the `static mut` reference.
    static IMAGE: RefCell<Option<Retained<NSImage>>> = const { RefCell::new(None) };
    static IMAGE_READY: Cell<bool> = const { Cell::new(false) };
}

#[cfg(target_os = "macos")]
fn default_image(mtm: MainThreadMarker) -> Retained<NSImage> {
    IMAGE_READY.with(|ready| {
        if !ready.get() {
            let image =
                crate::dc::icon::svg_bytes_to_bgra(ICON_SVG, ICON_PX, ICON_PX).map(|bgra| {
                    crate::platform::appkit::nsimage_from_bgra(
                        mtm,
                        &bgra,
                        ICON_PX as i32,
                        ICON_PX as i32,
                    )
                });
            IMAGE.with(|slot| *slot.borrow_mut() = image);
            ready.set(true);
        }
    });
    IMAGE.with(|slot| slot.borrow().clone()).unwrap_or_else(|| {
        NSImage::initWithSize(
            NSImage::alloc(),
            objc2_foundation::NSSize::new(ICON_PX as f64, ICON_PX as f64),
        )
    })
}

/// `HICON` pair for [`WNDCLASSEXW`](windows_sys::Win32::UI::WindowsAndMessaging::WNDCLASSEXW)
/// registration (`hIcon` / `hIconSm`). Handles are cached for the process lifetime.
#[cfg(target_os = "windows")]
pub fn class_icons() -> (HICON, HICON) {
    (default_big() as HICON, default_small() as HICON)
}

/// Apply the library default icon, or `custom` when provided.
///
/// `custom` is used for both large and small slots (typical when the
/// caller supplies a multi-size `.ico` resource). Pass `None` for the
/// built-in `ru_wx` logo.
#[cfg(target_os = "windows")]
#[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper; `hwnd` must be live
pub fn apply_to_hwnd(hwnd: HWND, custom: Option<HICON>) {
    if hwnd.is_null() {
        return;
    }
    let (big, small) = match custom {
        Some(h) if !h.is_null() => (h as NativeHandle, h as NativeHandle),
        _ => (default_big(), default_small()),
    };
    // SAFETY: `hwnd` is a live top-level window; `WM_SETICON` with
    // `ICON_BIG` / `ICON_SMALL` is the documented Win32 API.
    unsafe {
        SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, big);
        SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, small);
        InvalidateRect(hwnd, std::ptr::null(), 1);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn apply_to_hwnd(_hwnd: isize, _custom: Option<isize>) {}

/// Apply the library default icon, or a cached bitmap handle when provided (macOS).
#[cfg(target_os = "macos")]
pub fn apply_to_frame(frame_handle: NativeHandle, custom_bitmap: Option<isize>) {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let image = match custom_bitmap {
        Some(h) if h != 0 => crate::platform::appkit::mac_bitmap_image_at(h),
        _ => Some(default_image(mtm)),
    };
    if let Some(img) = image {
        crate::platform::appkit::mac_frame_set_icon(frame_handle, Some(&img));
    }
}

#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
pub fn apply_to_frame(_frame_handle: NativeHandle, _custom_bitmap: Option<isize>) {}

#[cfg(all(test, target_os = "windows"))]
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
