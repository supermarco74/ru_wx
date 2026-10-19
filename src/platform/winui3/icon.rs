//! Icon helpers: SVG → HICON, frame-icon, and tray-icon.
//!
//! Three public functions cover the WinUI3 use cases:
//!
//! * [`svg_to_hicon`] — parse an SVG byte slice and produce an `HICON`
//!   at a chosen pixel size using the same `resvg` pipeline already used
//!   by `crate::platform::winui3::app::convert_svg_to_bitmap`, but
//!   returning a Win32 icon handle instead of a raw bitmap.
//! * [`set_frame_icon`] — set the icon associated with a top-level
//!   frame (both the small and large variants).
//! * [`IconTray`] — a minimal wrapper that registers a `Shell_NotifyIconW`
//!   record and exposes a small helper to show / hide the tray icon.

#![allow(non_snake_case)]

use std::ptr;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::Shell::NOTIFYICONDATAW;
use windows_sys::Win32::UI::WindowsAndMessaging::{CreateIcon, *};

// ============================================================================
// Convert `svg_data` into an `HICON` of the requested pixel size.
//
// Internally delegates to `resvg` to render the SVG into a BGRA
// pixmap, then wraps it in a Win32 DIB-section icon via `CreateIcon`.
pub fn svg_to_hicon(svg_data: &[u8], size: u32) -> Option<HICON> {
    let rgba = svg_to_rgba(svg_data, size)?;
    Some(create_icon_from_rgba(&rgba, size, size))
}

fn svg_to_rgba(svg_data: &[u8], size: u32) -> Option<Vec<u8>> {
    let tree = resvg::usvg::Tree::from_data(svg_data, &resvg::usvg::Options::default()).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size)?;
    let svg_size = tree.size();
    let scale = (size as f32 / svg_size.width()).min(size as f32 / svg_size.height());
    let offset_x = (size as f32 - svg_size.width() * scale) / 2.0;
    let offset_y = (size as f32 - svg_size.height() * scale) / 2.0;
    let transform =
        resvg::tiny_skia::Transform::from_translate(offset_x, offset_y).post_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Some(pixmap.data().to_vec())
}

fn create_icon_from_rgba(rgba: &[u8], width: u32, height: u32) -> HICON {
    // RGBA -> BGRA + opaque mask: Win32 wants AND mask + XOR mask.
    // We use `CreateIcon` with one monochrome mask + one color mask.
    let mut bgra: Vec<u8> = Vec::with_capacity((width * height * 4) as usize);
    for i in 0..(width * height) as usize {
        let src = i * 4;
        bgra.push(rgba[src + 2]); // B
        bgra.push(rgba[src + 1]); // G
        bgra.push(rgba[src]); // R
        bgra.push(rgba[src + 3]); // A
    }
    let mask_size = ((width * height) / 8).max(1) as usize;
    let mut mask: Vec<u8> = vec![0; mask_size];
    for i in 0..(width * height) as usize {
        if rgba[i * 4 + 3] > 128 {
            mask[i / 8] |= 1 << (i % 8);
        }
    }
    // SAFETY: both buffers are live for the duration of the call; Win32
    // copies them into its own heap.
    unsafe {
        CreateIcon(
            ptr::null_mut(),
            width as i32,
            height as i32,
            1,
            32,
            mask.as_ptr() as *const _,
            bgra.as_ptr() as *const _,
        )
    }
}

// ============================================================================
// Apply an HICON to a top-level frame. Both the small and large variants
// are populated so the title-bar and the Alt-Tab thumbnail agree.
#[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper; handles must be live
pub fn set_frame_icon(hwnd: HWND, hicon: HICON) {
    if hicon.is_null() || hwnd.is_null() {
        return;
    }
    unsafe {
        SendMessageW(hwnd, WM_SETICON, ICON_SMALL, hicon as isize);
        SendMessageW(hwnd, WM_SETICON, ICON_BIG, hicon as isize);
    }
}

const ICON_SMALL: usize = 0;
const ICON_BIG: usize = 1;

// ============================================================================
// Icon tray.
//
// Wraps the Shell_NotifyIconW dance that adds a small icon to the
// notification area (system tray). The `show()`/`hide()` methods
// toggle the visibility, and `tooltip()` updates the tooltip text on
// the fly.
pub struct IconTray {
    hwnd: HWND,
    nid: NOTIFYICONDATAW,
    added: bool,
}

impl IconTray {
    /// Construct a tray icon bound to `hwnd` (the message owner).
    pub fn new(hwnd: HWND, icon: HICON, tooltip: &str) -> Self {
        let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_ICON | NIF_TIP | NIF_MESSAGE;
        nid.uCallbackMessage = WM_USER + 20; // user-defined
        nid.hIcon = icon;
        let wide = encode_wide(tooltip);
        let len = wide.len().min(nid.szTip.len());
        nid.szTip[..len].copy_from_slice(&wide[..len]);
        Self {
            hwnd,
            nid,
            added: false,
        }
    }

    pub fn show(&mut self) {
        if self.added {
            return;
        }
        unsafe {
            Shell_NotifyIconW(NIM_ADD, &self.nid);
        }
        self.added = true;
    }

    pub fn hide(&mut self) {
        if !self.added {
            return;
        }
        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &self.nid);
        }
        self.added = false;
    }

    pub fn tooltip(&mut self, text: &str) {
        let wide = encode_wide(text);
        let len = wide.len().min(self.nid.szTip.len());
        self.nid.szTip[..len].copy_from_slice(&wide[..len]);
        unsafe {
            Shell_NotifyIconW(NIM_MODIFY, &self.nid);
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}

impl Drop for IconTray {
    fn drop(&mut self) {
        self.hide();
    }
}

// Win32 constants that `windows-sys 0.59` does not re-export.
const NIM_ADD: u32 = 0x00000000;
const NIM_MODIFY: u32 = 0x00000001;
const NIM_DELETE: u32 = 0x00000002;
const NIF_ICON: u32 = 0x00000002;
const NIF_TIP: u32 = 0x00000004;
const NIF_MESSAGE: u32 = 0x00000001;

#[link(name = "shell32.dll", kind = "raw-dylib", modifiers = "+verbatim")]
unsafe extern "system" {
    fn Shell_NotifyIconW(dwMessage: u32, pnid: *const NOTIFYICONDATAW) -> BOOL;
}

fn encode_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
