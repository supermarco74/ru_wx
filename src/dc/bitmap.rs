//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Single-resolution bitmap (`wxBitmap`).
//!
//! A [`Bitmap`] is a thin owner of a single `HBITMAP` GDI
//! handle plus its pixel dimensions. Unlike
//! [`crate::dc::bitmap_bundle::BitmapBundle`], it does **not**
//! carry multiple rasterisations — it is the single-resolution
//! version of the same idea.
//!
//! # Win32 model
//!
//! On Windows the bitmap is an `HBITMAP` returned by
//! `CreateDIBSection` (for blank bitmaps) or borrowed from
//! somewhere else (for loaded images). The constructor takes
//! ownership; [`Drop`] frees the handle via `DeleteObject`.
//!
//! To load an image from a file (PNG / JPEG / BMP), use
//! [`crate::dc::image::Image::load_from_file`] and then convert
//! it to a `Bitmap` with [`Image::to_bitmap`](crate::Image::to_bitmap).
//!
//! # Cross-platform stub
//!
//! On non-Windows targets [`Bitmap`] is a pure data struct
//! (no real bitmap exists). The drawing surface is not
//! implemented off-Windows.

#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{
    CreateDIBSection, DeleteObject, GetDC, ReleaseDC, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS,
    HBITMAP,
};

/// Single-resolution bitmap.
#[derive(Debug, Clone)]
pub struct Bitmap {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `true` if the bitmap has been nullified by [`Bitmap::destroy`]
    /// or moved into another owner. Always `false` on
    /// non-Windows targets without a backing store.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    empty: bool,
    /// Win32 GDI handle or macOS bitmap-cache id.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    handle: isize,
}

impl Bitmap {
    /// Create a new blank bitmap of the given pixel
    /// dimensions. The pixels are initialised to opaque
    /// black (zeros). On Windows this is implemented with
    /// `CreateDIBSection`.
    #[cfg(target_os = "windows")]
    pub fn new(width: u32, height: u32) -> Self {
        // SAFETY: BITMAPINFO is a plain C struct; we
        // initialise every field, including the
        // 1-element bmiColors array which is unused
        // for a 32-bit DIB.
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: height as i32,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0, // BI_RGB
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            // SAFETY: `RGBQUAD` is a plain C struct, and
            // `bmiColors` is unused for a 32-bit DIB
            // (BI_RGB, no palette).
            bmiColors: [unsafe { std::mem::zeroed() }],
        };
        // SAFETY: FFI call; `bmi` is fully initialised,
        // `usage` is `DIB_RGB_COLORS`, the HDC / section /
        // offset arguments are unused for a non-section
        // DIB. `ppvbits` may be null.
        let handle = unsafe {
            let screen = GetDC(std::ptr::null_mut());
            let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
            let hbmp = CreateDIBSection(
                screen,
                &bmi,
                DIB_RGB_COLORS,
                &mut bits,
                std::ptr::null_mut(),
                0,
            );
            ReleaseDC(std::ptr::null_mut(), screen);
            hbmp
        };
        Self {
            width,
            height,
            empty: handle.is_null(),
            handle: handle as isize,
        }
    }

    /// Non-Windows stub: returns a width/height record
    /// with no real backing store.
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    #[cfg(target_os = "macos")]
    pub fn new(width: u32, height: u32) -> Self {
        let pixel_count = width as usize * height as usize;
        let bgra = vec![0u8; pixel_count * 4];
        let handle = crate::platform::appkit::mac_bitmap_store_bgra(&bgra, width, height);
        Self {
            width,
            height,
            empty: handle == 0,
            handle,
        }
    }

    #[cfg(target_os = "macos")]
    pub fn from_cache_handle(
        handle: crate::platform::NativeHandle,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            width,
            height,
            empty: handle == 0,
            handle,
        }
    }

    /// Rasterise embedded SVG bytes to a bitmap of the given size.
    pub fn from_svg_bytes(svg: &[u8], width: u32, height: u32) -> Self {
        #[cfg(target_os = "windows")]
        {
            if let Some(hb) = crate::dc::icon::svg_bytes_to_hbitmap(svg, width, height) {
                // SAFETY: `svg_bytes_to_hbitmap` returns a fresh handle we own.
                return unsafe { Self::from_hbitmap(hb, width, height) };
            }
            Self::new(width, height)
        }
        #[cfg(not(target_os = "windows"))]
        let _ = svg;
        #[cfg(target_os = "macos")]
        {
            if let Some(bgra) = crate::dc::icon::svg_bytes_to_bgra(svg, width, height) {
                let handle = crate::platform::appkit::mac_bitmap_store_bgra(&bgra, width, height);
                if handle != 0 {
                    return Self {
                        width,
                        height,
                        empty: false,
                        handle,
                    };
                }
            }
            return Self::new(width, height);
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        Self::new(width, height)
    }

    /// Take ownership of an existing `HBITMAP` (e.g. one
    /// produced by [`crate::dc::image::Image::to_bitmap`]).
    /// The handle is freed by [`Drop`].
    ///
    /// # Safety
    ///
    /// `handle` must either be `0` (in which case the
    /// resulting `Bitmap` is a "null" bitmap) or a valid
    /// `HBITMAP` that the caller is transferring
    /// ownership of. The handle must not be in use by
    /// another GDI object.
    #[cfg(target_os = "windows")]
    pub unsafe fn from_hbitmap(handle: HBITMAP, width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            empty: handle.is_null(),
            handle: handle as isize,
        }
    }

    /// Borrow the underlying native bitmap handle ([`NativeHandle`](crate::platform::NativeHandle)).
    /// Returns `0` after [`Bitmap::destroy`] or on non-Windows targets.
    pub fn handle(&self) -> crate::platform::NativeHandle {
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        {
            self.handle as crate::platform::NativeHandle
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            0
        }
    }

    pub fn is_null(&self) -> bool {
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        {
            self.empty
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            false
        }
    }

    #[cfg(target_os = "macos")]
    pub fn destroy(&mut self) {
        if !self.empty && self.handle != 0 {
            crate::platform::appkit::mac_bitmap_release(self.handle);
        }
        self.empty = true;
        self.handle = 0;
    }

    /// Free the underlying `HBITMAP`. Safe to call
    /// multiple times; subsequent calls are no-ops.
    /// Always called automatically by [`Drop`].
    #[cfg(target_os = "windows")]
    pub fn destroy(&mut self) {
        if !self.empty && self.handle != 0 {
            // SAFETY: `handle` was either created by
            // `CreateDIBSection` in `new` or transferred
            // into the `Bitmap` via `from_hbitmap`. We
            // own it. `DeleteObject` accepts any GDI
            // object handle.
            unsafe {
                DeleteObject(self.handle as HBITMAP);
            }
        }
        self.empty = true;
        self.handle = 0;
    }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
impl Drop for Bitmap {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        self.destroy();
        #[cfg(target_os = "macos")]
        self.destroy();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "windows")]
    use crate::platform::NativeHandleExt;

    /// Su macOS lo store bitmap richiede AppKit attivo sul main
    /// thread: sotto `cargo test` l'handle resta nullo.
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn new_blank_records_dimensions() {
        let bmp = Bitmap::new(32, 24);
        assert_eq!(bmp.width, 32);
        assert_eq!(bmp.height, 24);
        assert!(!bmp.is_null());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn new_blank_has_nonnull_handle() {
        let bmp = Bitmap::new(16, 16);
        assert!(!bmp.handle().is_null());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn destroy_nullifies_handle() {
        let mut bmp = Bitmap::new(8, 8);
        assert!(!bmp.is_null());
        bmp.destroy();
        assert!(bmp.is_null());
        assert!(bmp.handle().is_null());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn from_hbitmap_records_dimensions_and_is_non_null() {
        let src = Bitmap::new(10, 10);
        let raw = src.handle();
        // SAFETY: `raw` was created by `CreateDIBSection`
        // and is being transferred.
        let moved = unsafe { Bitmap::from_hbitmap(raw as HBITMAP, 10, 10) };
        assert_eq!(moved.width, 10);
        assert_eq!(moved.height, 10);
        assert!(!moved.is_null());
    }
}
