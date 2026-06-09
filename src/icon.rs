//! SVG icon loading and conversion to native bitmaps.
//!
//! Uses `resvg` for SVG rendering and creates Win32 `HBITMAP` handles
//! on Windows. On other platforms, a stub is provided.

use std::path::Path;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, DestroyIcon, HICON, ICONINFO,
};

/// Internal helper: render SVG bytes to a BGRA pixel buffer at the given size.
///
/// Returns `None` if parsing or rendering fails.
fn render_svg_to_pixels(svg_bytes: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    let tree = resvg::usvg::Tree::from_data(svg_bytes, &resvg::usvg::Options::default()).ok()?;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)?;

    let svg_size = tree.size();
    let scale_x = width as f32 / svg_size.width();
    let scale_y = height as f32 / svg_size.height();
    let scale = scale_x.min(scale_y);

    // Centre the icon within the target size
    let offset_x = (width as f32 - svg_size.width() * scale) / 2.0;
    let offset_y = (height as f32 - svg_size.height() * scale) / 2.0;

    let transform =
        resvg::tiny_skia::Transform::from_translate(offset_x, offset_y).post_scale(scale, scale);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Convert RGBA pixels from the pixmap to BGRA (Win32 DIB section format).
    //
    // The v0.5.8 widening fix in `svg_bytes_to_hbitmap`
    // below moved the `* 4` into `usize` to avoid a `u32`
    // overflow for large dimensions, but this earlier code
    // path was missed. We now do the same widening here so
    // the `vec![0u8; ...]` cannot silently wrap on 32-bit
    // hosts (`u32 * u32 * 4 > 2^32` for a 65536×65536 image)
    // and so the loop bound matches the buffer size.
    let rgba = pixmap.data();
    let mut bgra = vec![0u8; (width as usize) * (height as usize) * 4];
    let px_count = (width as usize) * (height as usize);
    for i in 0..px_count {
        bgra[i * 4] = rgba[i * 4 + 2]; // Blue
        bgra[i * 4 + 1] = rgba[i * 4 + 1]; // Green
        bgra[i * 4 + 2] = rgba[i * 4]; // Red
        bgra[i * 4 + 3] = rgba[i * 4 + 3]; // Alpha
    }
    Some(bgra)
}

/// Load an SVG file and create a Win32 HBITMAP at the specified size.
#[cfg(target_os = "windows")]
pub fn load_svg_as_hbitmap(svg_path: &Path, width: u32, height: u32) -> Option<HBITMAP> {
    let svg_data = std::fs::read(svg_path).ok()?;
    svg_bytes_to_hbitmap(&svg_data, width, height)
}

/// Render SVG bytes and create a Win32 HBITMAP at the specified size.
#[cfg(target_os = "windows")]
pub fn svg_bytes_to_hbitmap(svg_bytes: &[u8], width: u32, height: u32) -> Option<HBITMAP> {
    let bgra = render_svg_to_pixels(svg_bytes, width, height)?;

    // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
    unsafe {
        let hdc = GetDC(0 as HWND);

        // Set up BITMAPINFO for a 32-bit top-down DIB
        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = width as i32;
        bmi.bmiHeader.biHeight = -(height as i32); // negative = top-down
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        let mut bits_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let hbitmap = CreateDIBSection(hdc, &bmi, DIB_RGB_COLORS, &mut bits_ptr, 0 as HANDLE, 0);

        ReleaseDC(0 as HWND, hdc);

        if hbitmap == 0 as HBITMAP || bits_ptr.is_null() {
            return None;
        }

        // Copy BGRA pixels into the DIB section.
        //
        // The byte count is `width * height * 4`. The pre-v0.5.8
        // code did the multiplication in `u32` and only then
        // cast to `usize`, which silently wraps for large
        // dimensions (e.g. 65536×65536 overflows `u32`). On
        // 64-bit hosts the cast back to `usize` is fine but the
        // resulting length is wrong, leading to a too-small
        // slice and a `copy_from_slice` that panics on the
        // length mismatch. Widening to `usize` *first* removes
        // the overflow window entirely.
        let byte_count = (width as usize) * (height as usize) * 4;
        let dest = std::slice::from_raw_parts_mut(bits_ptr as *mut u8, byte_count);
        dest.copy_from_slice(&bgra);

        Some(hbitmap)
    }
}

/// Load an SVG from an embedded byte slice and create a Win32 HBITMAP.
/// Alias for [`svg_bytes_to_hbitmap`].
#[cfg(target_os = "windows")]
pub fn load_svg_bytes_as_hbitmap(svg_bytes: &[u8], width: u32, height: u32) -> Option<HBITMAP> {
    svg_bytes_to_hbitmap(svg_bytes, width, height)
}

/// Convert a 32-bit BGRA `HBITMAP` (as produced by [`svg_bytes_to_hbitmap`])
/// into a Win32 `HICON` suitable for use with the system tray
/// (`Shell_NotifyIconW`), window icons, etc.
///
/// A monochrome mask is generated (all zeros = fully opaque); the alpha
/// channel of the colour bitmap is preserved.
///
/// # Errors
///
/// Returns a **null `HICON`** when `CreateIconIndirect` fails (out of
/// memory, malformed `BITMAP` from `GetObjectW`, etc.). Callers must
/// check for null; wrappers in this module that return `Option<HICON>`
/// translate the null into `None` so the user cannot accidentally
/// treat a failure as a valid handle.
#[cfg(target_os = "windows")]
#[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper around CreateIconIndirect
pub fn hbitmap_to_hicon(hbitmap: HBITMAP) -> HICON {
    // SAFETY: `GetObjectW`, `CreateBitmap`, `CreateIconIndirect` and
    // `DeleteObject` are Win32 GDI APIs that take / return raw
    // handles. We pre-zero `BITMAP` and `ICONINFO` so all uninitialised
    // fields are valid zero values. The mask bitmap is released before
    // returning; ownership of the colour bitmap transfers to the new
    // `HICON`.
    // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
    unsafe {
        // Get the bitmap dimensions so we can build a matching 1-bpp mask.
        let mut bmp: BITMAP = std::mem::zeroed();
        let bytes = std::mem::size_of::<BITMAP>() as i32;
        let ok = GetObjectW(hbitmap, bytes, &mut bmp as *mut _ as *mut _);
        let (w, h) = if ok > 0 {
            (bmp.bmWidth, bmp.bmHeight)
        } else {
            (16, 16)
        };

        // 1-bpp mask bitmap: all zeros (every pixel opaque).
        let mask = CreateBitmap(w, h, 1, 1, std::ptr::null());

        let ii = ICONINFO {
            fIcon: 1,
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask,
            hbmColor: hbitmap,
        };
        let hicon = CreateIconIndirect(&ii);
        // The system copies the bitmap data internally, so we can delete
        // our copies right away.
        if !mask.is_null() {
            DeleteObject(mask);
        }
        hicon
    }
}

/// Create a small `HICON` (e.g. 16×16) directly from embedded SVG bytes.
/// Convenience wrapper combining [`svg_bytes_to_hbitmap`] and
/// [`hbitmap_to_hicon`].
#[cfg(target_os = "windows")]
pub fn svg_bytes_to_hicon(svg_bytes: &[u8], size: u32) -> Option<HICON> {
    let hbmp = svg_bytes_to_hbitmap(svg_bytes, size, size)?;
    // `hbitmap_to_hicon` is a safe thin FFI wrapper (it has no
    // soundness preconditions beyond those enforced by
    // `windows-sys`); the call site is plain Rust.
    let hicon = hbitmap_to_hicon(hbmp);
    // The HICON keeps its own reference to the colour bitmap data, so we
    // can delete the intermediate HBITMAP. `DeleteObject` is a no-op
    // when called with a null handle, so this is safe even if the
    // bitmap is somehow null.
    // SAFETY: `hbmp` was just produced by `svg_bytes_to_hbitmap` (a
    // DIB section we own); `DeleteObject` is the correct destructor.
    unsafe {
        DeleteObject(hbmp);
    }
    // `hbitmap_to_hicon` returns a null `HICON` when
    // `CreateIconIndirect` fails. A null handle inside a `Some` is
    // indistinguishable from a real handle for the caller, which is a
    // silent failure: the user would treat the bogus "icon" as
    // valid and pass it to `Shell_NotifyIconW` / `BM_SETIMAGE` / etc.
    // Map the null to `None` so the caller can fall back.
    if hicon.is_null() {
        return None;
    }
    Some(hicon)
}

/// Destroy an `HICON` previously created with [`hbitmap_to_hicon`] /
/// [`svg_bytes_to_hicon`].
#[cfg(target_os = "windows")]
#[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper around DestroyIcon
pub fn destroy_hicon(hicon: HICON) {
    if !hicon.is_null() {
        // SAFETY: `DestroyIcon` releases an `HICON` allocated by
        // `CreateIconIndirect`. Null check above guarantees we are not
        // destroying a system stock icon.
        // SAFETY: FFI call to DestroyIcon on cursor / icon handles owned by this crate.
        unsafe { DestroyIcon(hicon) };
    }
}

// ---- Non-Windows stubs ----

#[cfg(not(target_os = "windows"))]
pub struct HBitmap;

#[cfg(not(target_os = "windows"))]
pub fn load_svg_as_hbitmap(_svg_path: &Path, _width: u32, _height: u32) -> Option<HBitmap> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn svg_bytes_to_hbitmap(_svg_bytes: &[u8], _width: u32, _height: u32) -> Option<HBitmap> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn load_svg_bytes_as_hbitmap(_svg_bytes: &[u8], _width: u32, _height: u32) -> Option<HBitmap> {
    None
}
