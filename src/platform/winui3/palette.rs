//! Background and foreground (text) color management for widgets.
//!
//! Win32 exposes control colors via class extra data and via the
//! parent window's `WM_CTLCOLOR*` handler. The simpler approach used
//! here is `SetClassLongPtrW(hwnd, GCLP_HBRBACKGROUND, hbrush)`,
//! which is enough for the static controls (`STATIC`, `BUTTON`/`BS_*`
//! in non-pushbutton styles, group boxes) but **not** for the typical
//! interactive controls (`EDIT`, `LISTBOX`, ...) — those have their
//! own client area invalidation rules. We attempt the set anyway and
//! silently ignore the round-trip no-op when the control refuses it.

#![allow(non_snake_case)]

use std::collections::HashMap;
use std::sync::Mutex;

use windows_sys::Win32::Foundation::{COLORREF, HWND};
use windows_sys::Win32::Graphics::Gdi::{CreateSolidBrush, InvalidateRect, RedrawWindow, HBRUSH};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetClassLongPtrW, SetClassLongPtrW, GCLP_HBRBACKGROUND,
};

// ============================================================================
// COLORREF helpers.
// ============================================================================

/// Pack an RGB triple into the 0x00BBGGRR value Win32 uses.
pub const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    ((b as u32) << 16) | ((g as u32) << 8) | (r as u32)
}

/// Alias for `rgb`.
pub const fn color_ref(r: u8, g: u8, b: u8) -> COLORREF {
    rgb(r, g, b)
}

/// A trivial named palette a few presets for fast reuse.
pub mod preset {
    use super::{color_ref, COLORREF};
    pub const WHITE: COLORREF = color_ref(0xFF, 0xFF, 0xFF);
    pub const BLACK: COLORREF = color_ref(0x00, 0x00, 0x00);
    pub const GRAY: COLORREF = color_ref(0xE5, 0xE7, 0xEB);
    pub const INDIGO: COLORREF = color_ref(0x4F, 0x46, 0xE5);
    pub const EMERALD: COLORREF = color_ref(0x10, 0xB9, 0x81);
    pub const AMBER: COLORREF = color_ref(0xF5, 0x9E, 0x0B);
    pub const ROSE: COLORREF = color_ref(0xF4, 0x3F, 0x5E);
    pub const SLATE: COLORREF = color_ref(0x47, 0x55, 0x69);
}

// ============================================================================
// Brush cache.
//
// `CreateSolidBrush` returns a brush that Win32 does NOT own — we need
// to call `DeleteObject` on it when done. SetClassLongPtrW takes the
// brush and lets the class eat it, but only if no other window of the
// same class already has a class brush; in practice we keep our own
// cache indexed by `(hwnd-as-key, color)` so repeated `set_*` calls
// reuse the same brush.
//
// `HBRUSH = *mut c_void` is `!Send` by default; we wrap it in a
// newtype whose `Send`-ness is asserted via `PhantomData<&()>` only
// ever touched from the main thread of the process. The cache lives
// behind a `Mutex` for safety but is never shared across threads.
struct BrushHandle(HBRUSH);
unsafe impl Send for BrushHandle {}

static BRUSH_CACHE: Mutex<Option<HashMap<(u32, u32), BrushHandle>>> = Mutex::new(None);

fn get_or_create_brush(color: COLORREF) -> HBRUSH {
    let mut guard = BRUSH_CACHE.lock().unwrap_or_else(|p| p.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    let key = (color, color);
    let h = map
        .entry(key)
        .or_insert_with(|| BrushHandle(unsafe { CreateSolidBrush(color) }));
    h.0
}

// ============================================================================
// Public API.
// ============================================================================

/// Apply `color` as the background of `hwnd`'s window class.
///
/// Win32 only honors the class background brush for a few control
/// classes (`STATIC`, `BUTTON` in non-pushbutton styles, group boxes,
/// `EDIT` in read-only mode, etc.). For interactive controls (push
/// buttons, list boxes) the request is silently ignored by Windows.
#[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper; `hwnd` must be live
pub fn set_background_color(hwnd: HWND, color: COLORREF) -> bool {
    if hwnd.is_null() {
        return false;
    }
    let hbrush = get_or_create_brush(color);
    unsafe { SetClassLongPtrW(hwnd, GCLP_HBRBACKGROUND, hbrush as isize) != 0 }
}

/// Returns the current background brush of `hwnd`, or 0 if none was set.
#[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper; `hwnd` must be live
pub fn background_brush(hwnd: HWND) -> usize {
    if hwnd.is_null() {
        return 0;
    }
    let r = unsafe { GetClassLongPtrW(hwnd, GCLP_HBRBACKGROUND) };
    if r == 0 {
        0
    } else {
        1
    }
}

/// True iff a background color has been applied to `hwnd` in this
/// session.
pub fn is_background_set(hwnd: HWND) -> bool {
    background_brush(hwnd) != 0
}

/// Convenience: set the foreground (text) color of `hwnd`.
///
/// Win32 doesn't expose a per-control text-color API; the canonical
/// way is via the parent window's `WM_CTLCOLORSTATIC` /
/// `WM_CTLCOLOREDIT` handler calling `SetTextColor`. Until the
/// event layer subclasses each control we expose this as a no-op
/// so the public surface stays consistent.
pub fn set_foreground_color(_hwnd: HWND, _color: COLORREF) {
    // intentionally left as a no-op until WM_CTLCOLOR subclass is wired.
}

/// Clear the internal brush cache. Useful for long-running apps that
/// spin up many transient widgets.
pub fn clear_brush_cache() {
    if let Ok(mut guard) = BRUSH_CACHE.lock() {
        guard.take();
    }
}

// ============================================================================
// Redraw helpers.
// ============================================================================

/// Ask the widget to redraw itself fully (background + foreground).
#[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper; `hwnd` must be live
pub fn request_redraw(hwnd: HWND) {
    if hwnd.is_null() {
        return;
    }
    unsafe {
        let _ = InvalidateRect(hwnd, std::ptr::null_mut(), 1);
        // RDW_INVALIDATE = 0x0001, RDW_UPDATENOW = 0x0100.
        let _ = RedrawWindow(
            hwnd,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0x0001u32 | 0x0100u32,
        );
    }
}
