//! Single-resolution bitmap (`wxBitmap`).
//!
//! A [`Bitmap`] is a thin owner of a single `HBITMAP` GDI
//! handle plus its pixel dimensions. Unlike
//! [`crate::bitmap_bundle::BitmapBundle`], it does **not**
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
//! [`crate::image::Image::load_from_file`] and then convert
//! it to a `Bitmap` with [`Image::to_bitmap`].
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
    /// non-Windows targets.
    #[cfg(target_os = "windows")]
    empty: bool,
    /// Win32 GDI handle. `0` after [`Bitmap::destroy`] or on
    /// non-Windows targets. We do not store `HBITMAP` directly
    /// because `#[derive(Clone)]` would otherwise require a
    /// `Copy` impl on the underlying pointer.
    #[cfg(target_os = "windows")]
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
    #[cfg(not(target_os = "windows"))]
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Take ownership of an existing `HBITMAP` (e.g. one
    /// produced by [`crate::image::Image::to_bitmap`]).
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

    /// Borrow the underlying `HBITMAP` handle. Returns
    /// `0` after [`Bitmap::destroy`] or on non-Windows
    /// targets. The handle is *borrowed* — do not call
    /// `DeleteObject` on it, [`Bitmap::destroy`] / [`Drop`]
    /// own the lifetime.
    #[cfg(target_os = "windows")]
    pub fn handle(&self) -> HBITMAP {
        self.handle as HBITMAP
    }

    /// Returns `true` if this bitmap has been destroyed
    /// (or never owned a real handle).
    pub fn is_null(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            self.empty
        }
        #[cfg(not(target_os = "windows"))]
        {
            true
        }
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

#[cfg(target_os = "windows")]
impl Drop for Bitmap {
    fn drop(&mut self) {
        self.destroy();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let moved = unsafe { Bitmap::from_hbitmap(raw, 10, 10) };
        assert_eq!(moved.width, 10);
        assert_eq!(moved.height, 10);
        assert!(!moved.is_null());
    }
}
