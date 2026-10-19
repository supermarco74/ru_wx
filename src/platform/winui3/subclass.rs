//! Window subclassing via `SetWindowSubclass` from `comctl32.dll`.
//!
//! The win32 UI toolkit uses `WM_CTLCOLOR*` messages from the parent
//! window to give every child control its colors and brushes. To make
//! the controls look like the rest of the modern UI (e.g. hyperlink
//! links shown in blue), we install a single `SUBCLASSPROC` on the
//! top-level frame that intercepts `WM_CTLCOLORSTATIC`, `WM_CTLCOLORBTN`
//! and `WM_CTLCOLOREDIT` and applies per-widget hints.

#![allow(non_snake_case)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};

// ============================================================================
// Per-window subclass context.
//
// One subclass context per top-level frame. The `SUBCLASSPROC` callback
// receives a `u32 subclass_id` that we use to recover the typed pointer.
// ============================================================================

/// Subclass id we use to look up the per-frame context.
const FRAME_SUBCLASS_ID: usize = 0xC0DE_FEED;

/// Per-HWND renderer hints. Keys are *child* HWNDs (the ones that
/// sent `WM_CTLCOLOR*`), values are the desired foreground colour.
type HintMap = HashMap<HWND, u32>;

thread_local! {
    static HINTS: RefCell<HintMap> = RefCell::new(HashMap::new());
}

extern "system" {}

// Note: comctl32's `SetWindowSubclass` / `RemoveWindowSubclass` come
// through the `windows-sys` bindings in `Win32::UI::Shell`. The
// signatures are:
//
//     type SUBCLASSPROC = Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM, usize, usize) -> LRESULT>;
//     fn SetWindowSubclass(hwnd, pSubclass: SUBCLASSPROC, uIdSubclass: usize, dwRefData: usize) -> BOOL;
//
// We pass our `frame_subclass_proc_transmute` as the `SUBCLASSPROC`.
//
// Hyperlink colour (≈ #0066CC, the canonical Win11 accent).
const HYPERLINK_COLOR: u32 = 0x00CC6600; // 0x00BBGGRR

unsafe extern "system" fn frame_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    uid: usize,
    _refdata: usize,
) -> LRESULT {
    if uid != FRAME_SUBCLASS_ID {
        return DefSubclassProc(hwnd, msg, wparam, lparam);
    }
    let msg_u32 = msg;
    const WM_CTLCOLORBTN: u32 = 0x0135;
    const WM_CTLCOLORSTATIC: u32 = 0x0138;
    const WM_CTLCOLOREDIT: u32 = 0x0133;
    if msg_u32 == WM_CTLCOLORBTN || msg_u32 == WM_CTLCOLORSTATIC || msg_u32 == WM_CTLCOLOREDIT {
        let child = lparam as HWND;
        let color = HINTS.with(|h| h.borrow().get(&child).copied()).unwrap_or(0);
        if color != 0 {
            unsafe {
                use windows_sys::Win32::Graphics::Gdi::SetTextColor;
                let hdc = wparam as _;
                SetTextColor(hdc, color);
                // Return a null brush pointer cast to LRESULT — `HBRUSH = *mut c_void`.
                // We avoid the `static HBRUSH` because raw pointers don't satisfy Sync.
                let null_brush = std::ptr::null_mut::<c_void>();
                return null_brush as isize;
            }
        }
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}

/// Install the frame-wide subclass on the given frame.
///
/// Has no effect if the subclass is already installed.
#[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper; `hwnd` must be live
pub fn install_frame_subclass(hwnd: HWND) {
    unsafe {
        // SetWindowSubclass returns TRUE / FALSE; we ignore errors.
        let _ = SetWindowSubclass(
            hwnd,
            Some(frame_subclass_proc_transmute as _),
            FRAME_SUBCLASS_ID,
            0,
        );
    }
}

/// Remove the subclass (call this from `WM_NCDESTROY`).
#[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper; `hwnd` must be live
pub fn uninstall_frame_subclass(hwnd: HWND) {
    unsafe {
        let _ = RemoveWindowSubclass(
            hwnd,
            Some(frame_subclass_proc_transmute as _),
            FRAME_SUBCLASS_ID,
        );
    }
}

// SetWindowSubclass takes an `unsafe extern "system" fn`; we wrap our
// closure with a transmute shim to satisfy the signature.
unsafe extern "system" fn frame_subclass_proc_transmute(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    uid: usize,
    refdata: usize,
) -> LRESULT {
    frame_subclass_proc(hwnd, msg, wparam, lparam, uid, refdata)
}

// ============================================================================
// Public per-widget hooks.
// ============================================================================

/// Set the foreground colour that the subclass will return for a given
/// child control. The colour is encoded as Win32 `0x00BBGGRR` (use
/// [`crate::platform::winui3::palette::rgb`] to build it).
pub fn set_foreground(child: HWND, color: u32) {
    HINTS.with(|h| {
        h.borrow_mut().insert(child, color);
    });
}

/// Reset the foreground hint for the given child.
pub fn clear_foreground(child: HWND) {
    HINTS.with(|h| {
        h.borrow_mut().remove(&child);
    });
}

/// Pre-built constants matching `palette::preset::*` so the user does
/// not need to import two modules just to colourise a Hyperlink.
pub const HYPERLINK_BLUE: u32 = HYPERLINK_COLOR;
