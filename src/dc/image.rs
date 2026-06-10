//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! `wxImage` — image container with format-independent pixel access.
//!
//! An [`Image`] holds a decoded raster image in memory as a
//! packed RGBA8 buffer, together with its pixel dimensions. It
//! is the format-independent counterpart of [`crate::dc::bitmap::Bitmap`]:
//!
//! - `Image` knows about pixels and can be saved, copied,
//!   resized (manually), queried for pixel data, and converted
//!   to a `Bitmap`.
//! - `Bitmap` is a thin owner of a single `HBITMAP` GDI handle
//!   and is the thing you hand to drawing code.
//!
//! # Supported source formats
//!
//! On all targets [`Image::load_from_memory`] (and
//! [`Image::load_from_file`]) can decode the formats enabled
//! at compile time in the `image` crate. By default
//! `ru_wx` enables BMP, PNG and JPEG.
//!
//! # Win32 conversion
//!
//! [`Image::to_bitmap`] converts the in-memory RGBA buffer
//! into a `Bitmap` (a Win32 `HBITMAP`) suitable for use with
//! [`crate::Dc`]. The pixel data is swizzled from RGBA
//! to BGRA (the layout Win32 expects for 32-bit DIBs).
//!
//! # Cross-platform stub
//!
//! On non-Windows targets the type is still usable — you can
//! load images and inspect their pixel data — but
//! [`Image::to_bitmap`] returns a stub `Bitmap` (no real
//! backing store) because there is no GDI.

use crate::dc::bitmap::Bitmap;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{
    CreateDIBSection, GetDC, ReleaseDC, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS,
};

/// 8-bit RGBA pixel, stored as `(R, G, B, A)` in memory.
pub type Rgba = (u8, u8, u8, u8);

/// Maximum pixel count we will allocate for an [`Image`].
///
/// The byte count is `4 * MAX_IMAGE_PIXELS` (one byte per
/// RGBA channel), capped at 256 MiB on 64-bit hosts. Anything
/// larger would either:
///
/// * overflow `usize` on 32-bit hosts (`u32 * u32 * 4 > 2^32`), or
/// * allocate gigabytes of RAM for a single image, which is
///   almost certainly a misuse (no real GUI needs a 100k×100k
///   raster).
///
/// When `Image::new` is called with dimensions that would
/// exceed this cap, the constructed image has a zero-size
/// pixel buffer and `is_null()` returns `true` so callers can
/// detect the rejection.
pub const MAX_IMAGE_PIXELS: usize = 64 * 1024 * 1024; // 64 M pixels = 256 MiB

/// Compute the byte count for a given `(width, height)`,
/// returning `None` on overflow or when the result would
/// exceed [`MAX_IMAGE_PIXELS`]. Centralises the overflow
/// logic so every code path uses the same rule.
#[inline]
fn checked_image_byte_count(width: u32, height: u32) -> Option<usize> {
    let pixels = (width as usize).checked_mul(height as usize)?;
    if pixels > MAX_IMAGE_PIXELS {
        return None;
    }
    pixels.checked_mul(4)
}

/// Errors that can occur when working with [`Image`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageError {
    /// The file could not be read from disk.
    IoError(String),
    /// The data could not be decoded by the underlying image
    /// library (e.g. unsupported format, corrupt data).
    DecodeError(String),
    /// The image has zero width or height.
    InvalidSize,
}

impl std::fmt::Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageError::IoError(s) => write!(f, "i/o error: {s}"),
            ImageError::DecodeError(s) => write!(f, "decode error: {s}"),
            ImageError::InvalidSize => write!(f, "invalid image size (zero width or height)"),
        }
    }
}

impl std::error::Error for ImageError {}

/// Format-independent image. Holds an RGBA8 pixel buffer and
/// the dimensions of the image.
#[derive(Debug, Clone)]
pub struct Image {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// RGBA8 pixel data, row-major, top-to-bottom, no padding.
    /// Length is always `width * height * 4` bytes.
    pixels: Vec<u8>,
}

/// Compute the byte index of pixel `(x, y)` for a
/// row-major image of `width` pixels, returning `None` on
/// `usize` overflow. The 4-byte RGBA stride is multiplied
/// in last so that we fail fast on the smaller operands.
#[inline]
fn pixel_index(y: u32, width: u32, x: u32) -> Option<usize> {
    let row = (y as usize).checked_mul(width as usize)?;
    let col = row.checked_add(x as usize)?;
    col.checked_mul(4)
}

impl Image {
    /// Create an empty image of the given dimensions filled
    /// with opaque black. Mainly useful for tests and for
    /// building images pixel-by-pixel.
    ///
    /// Returns a *null* image (zero-size pixel buffer) when
    /// the requested dimensions would either overflow
    /// `usize` or exceed [`MAX_IMAGE_PIXELS`]. Callers can
    /// detect the rejection with [`Image::is_null`].
    pub fn new(width: u32, height: u32) -> Self {
        let pixels = checked_image_byte_count(width, height)
            .map(|n| vec![0u8; n])
            .unwrap_or_default();
        Self {
            width,
            height,
            pixels,
        }
    }

    /// Create an image from a raw RGBA8 pixel buffer. The
    /// buffer length must be exactly `width * height * 4`
    /// bytes. Used by the GIF/APNG decoder in
    /// [`crate::adv::animation::Animation`] and by anyone who needs
    /// to build an `Image` from existing RGBA data.
    ///
    /// The buffer is *clamped* (truncated or zero-extended
    /// as needed) to the size implied by `width * height`.
    /// The previous implementation stored the buffer as-is,
    /// so a too-small or too-large input silently desynced
    /// `pixels().len()` from `width * height * 4` and
    /// corrupted the bounds check in `get_pixel` / `set_pixel`.
    pub fn from_rgba8(width: u32, height: u32, mut pixels: Vec<u8>) -> Self {
        // Reject dimensions that would overflow usize or
        // exceed MAX_IMAGE_PIXELS by collapsing the buffer
        // to a zero-size placeholder — `is_null()` will then
        // report the failure.
        let Some(expected) = checked_image_byte_count(width, height) else {
            return Self {
                width,
                height,
                pixels: Vec::new(),
            };
        };
        if pixels.len() != expected {
            pixels.resize(expected, 0);
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    /// Decode an image from a byte buffer. The format is
    /// auto-detected by the `image` crate.
    pub fn load_from_memory(data: &[u8]) -> Result<Self, ImageError> {
        let dyn_img = image::load_from_memory(data)
            .map_err(|e| ImageError::DecodeError(e.to_string()))?;
        Ok(Self::from_dynamic_image(dyn_img))
    }

    /// Decode an image from a file on disk. The format is
    /// auto-detected by the file extension.
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, ImageError> {
        let data = std::fs::read(path).map_err(|e| ImageError::IoError(e.to_string()))?;
        Self::load_from_memory(&data)
    }

    /// Convert an [`image::DynamicImage`] to an [`Image`],
    /// always flattening the result to RGBA8.
    fn from_dynamic_image(dyn_img: image::DynamicImage) -> Self {
        let rgba = dyn_img.to_rgba8();
        let (w, h) = rgba.dimensions();
        Self {
            width: w,
            height: h,
            pixels: rgba.into_raw(),
        }
    }

    /// Returns the raw RGBA8 pixel buffer. The buffer length
    /// is `width * height * 4` bytes, with the same layout as
    /// `image::RgbaImage::into_raw`.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Returns a mutable view of the pixel buffer.
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }

    /// Returns the RGBA8 pixel at `(x, y)`, or `None` if the
    /// coordinates are out of range or the index would
    /// overflow `usize` (the latter is unreachable for
    /// images constructed via [`Image::new`] / [`Image::from_rgba8`]
    /// but we guard it defensively).
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<Rgba> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = pixel_index(y, self.width, x)?;
        let end = idx.checked_add(4)?;
        if end > self.pixels.len() {
            return None;
        }
        Some((self.pixels[idx], self.pixels[idx + 1], self.pixels[idx + 2], self.pixels[idx + 3]))
    }

    /// Set the RGBA8 pixel at `(x, y)`. Returns `false` if
    /// the coordinates are out of range or the index would
    /// overflow `usize`.
    pub fn set_pixel(&mut self, x: u32, y: u32, pixel: Rgba) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let Some(idx) = pixel_index(y, self.width, x) else {
            return false;
        };
        let Some(end) = idx.checked_add(4) else {
            return false;
        };
        if end > self.pixels.len() {
            return false;
        }
        self.pixels[idx] = pixel.0;
        self.pixels[idx + 1] = pixel.1;
        self.pixels[idx + 2] = pixel.2;
        self.pixels[idx + 3] = pixel.3;
        true
    }

    /// Returns `true` if the image is empty (zero width or
    /// height, or no pixels).
    pub fn is_null(&self) -> bool {
        self.width == 0 || self.height == 0 || self.pixels.is_empty()
    }

    /// Convert this [`Image`] into a [`Bitmap`], allocating a
    /// Win32 32-bit DIB and copying the pixels. The pixel
    /// data is swizzled from RGBA to BGRA (the layout Win32
    /// expects for 32-bit `BI_RGB` DIBs).
    #[cfg(target_os = "windows")]
    pub fn to_bitmap(&self) -> Bitmap {
        if self.is_null() {
            return Bitmap::new(self.width, self.height);
        }
        // SAFETY: BITMAPINFO is a plain C struct; we
        // initialise every field.
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: self.width as i32,
                biHeight: -(self.height as i32), // top-down DIB: positive = bottom-up
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
        // `usage` is `DIB_RGB_COLORS`. `ppvbits` will receive
        // a pointer to the DIB's pixel storage (a buffer of
        // `width * height * 4` bytes). We write to it below.
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let hbmp = unsafe {
            let screen = GetDC(std::ptr::null_mut());
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
        if hbmp.is_null() {
            return Bitmap::new(self.width, self.height);
        }
        // SAFETY: `bits` points to a buffer of at least
        // `width * height * 4` bytes (the size of a 32-bit
        // DIB at those dimensions). We have just allocated it
        // via `CreateDIBSection`. The buffer is not aliased.
        unsafe {
            let dst = std::slice::from_raw_parts_mut(bits as *mut u8, self.pixels.len());
            for (src_px, dst_px) in self.pixels.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
                // RGBA -> BGRA: swap first and third bytes.
                dst_px[0] = src_px[2]; // B
                dst_px[1] = src_px[1]; // G
                dst_px[2] = src_px[0]; // R
                dst_px[3] = src_px[3]; // A
            }
        }
        // SAFETY: `hbmp` is a freshly-created DIB section,
        // we own it, and it is not aliased.
        unsafe { Bitmap::from_hbitmap(hbmp, self.width, self.height) }
    }

    /// Non-Windows stub. Returns a width/height stub.
    #[cfg(not(target_os = "windows"))]
    pub fn to_bitmap(&self) -> Bitmap {
        Bitmap::new(self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_records_dimensions_and_zeros() {
        let img = Image::new(8, 4);
        assert_eq!(img.width, 8);
        assert_eq!(img.height, 4);
        assert_eq!(img.pixels().len(), 8 * 4 * 4);
        assert!(img.pixels().iter().all(|&b| b == 0));
        assert!(!img.is_null());
    }

    #[test]
    fn get_set_pixel_round_trips() {
        let mut img = Image::new(4, 4);
        assert!(img.set_pixel(2, 3, (10, 20, 30, 255)));
        assert_eq!(img.get_pixel(2, 3), Some((10, 20, 30, 255)));
        // Out-of-range coordinates return None / false.
        assert_eq!(img.get_pixel(4, 0), None);
        assert!(!img.set_pixel(4, 0, (0, 0, 0, 0)));
    }

    #[test]
    fn is_null_for_zero_size() {
        let img = Image::new(0, 0);
        assert!(img.is_null());
    }

    #[test]
    fn load_from_memory_rejects_garbage() {
        let res = Image::load_from_memory(&[0u8, 1, 2, 3, 4, 5]);
        assert!(res.is_err());
    }

    #[test]
    fn to_bitmap_returns_correct_dimensions() {
        let img = Image::new(16, 12);
        let bmp = img.to_bitmap();
        assert_eq!(bmp.width, 16);
        assert_eq!(bmp.height, 12);
        #[cfg(target_os = "windows")]
        assert!(!bmp.is_null());
    }

    // ── v0.6.1 security tests ───────────────────────────────────────
    //
    // These exercise the overflow / DoS paths of the
    // pixel-buffer construction and indexing logic. The
    // pre-v0.6.1 code computed `width * height * 4` in
    // `usize` without any check, so a 65536×65536 request
    // either allocated gigabytes of zeroed memory (DoS) or
    // wrapped the length on 32-bit hosts (panic in
    // `vec![0u8; wrapped]`). We now reject anything above
    // `MAX_IMAGE_PIXELS`.

    #[test]
    fn new_rejects_dimensions_over_max_pixels() {
        // 8000 × 8000 = 64 000 000 pixels, just under the
        // 64 Mi cap (67 108 864), so this *should* succeed.
        let ok = Image::new(8000, 8000);
        assert!(!ok.is_null());
        assert_eq!(ok.pixels().len(), 8000 * 8000 * 4);

        // 9000 × 9000 = 81 000 000 pixels, above the 64 Mi
        // cap. Must collapse to a null image instead of
        // panicking.
        let too_big = Image::new(9000, 9000);
        assert!(too_big.is_null());
        assert_eq!(too_big.pixels().len(), 0);
        // Width/height are still recorded so the caller can
        // tell *what* the rejected request was.
        assert_eq!(too_big.width, 9000);
        assert_eq!(too_big.height, 9000);
    }

    #[test]
    fn new_rejects_32bit_overflow_dimensions() {
        // 65536 × 65536 × 4 = 2^34, which would overflow
        // `usize` on 32-bit hosts and silently wrap to
        // a tiny (wrong) buffer on 64-bit hosts. Either
        // way, we want a null image rather than a bad
        // allocation. (On 64-bit `usize` the wrap would
        // not happen but the byte count (16 GiB) is
        // still well above the cap, so the cap rejects
        // it; on 32-bit the `checked_mul` chain rejects
        // it earlier.)
        let img = Image::new(65_536, 65_536);
        assert!(img.is_null());
        assert_eq!(img.pixels().len(), 0);
    }

    #[test]
    fn from_rgba8_clamps_buffer_to_expected_size() {
        // Buffer too small for the declared dimensions:
        // we zero-extend it to the expected length so the
        // invariant `pixels.len() == width * height * 4`
        // is preserved.
        let tiny = vec![0xAAu8; 4];
        let img = Image::from_rgba8(2, 2, tiny);
        assert_eq!(img.pixels().len(), 2 * 2 * 4);
        assert!(!img.is_null());

        // Buffer too large for the declared dimensions:
        // we truncate it.
        let big = vec![0xBBu8; 2 * 2 * 4 + 16];
        let img2 = Image::from_rgba8(2, 2, big);
        assert_eq!(img2.pixels().len(), 2 * 2 * 4);
    }

    #[test]
    fn from_rgba8_rejects_oversize_dimensions() {
        // Same overflow case as `new_rejects_32bit_overflow_dimensions`,
        // but going through the `from_rgba8` entry point.
        let big_buf = vec![0u8; 1024];
        let img = Image::from_rgba8(65_536, 65_536, big_buf);
        assert!(img.is_null());
    }

    #[test]
    fn set_pixel_does_not_panic_on_oversize_dimensions() {
        // A pathological image must still be safe to call
        // `get_pixel` / `set_pixel` on — the method should
        // return None / false rather than panic on an
        // out-of-bounds access.
        let mut img = Image::new(65_536, 65_536);
        assert!(img.is_null());
        assert_eq!(img.get_pixel(0, 0), None);
        assert!(!img.set_pixel(0, 0, (0, 0, 0, 0)));
    }

    #[test]
    fn max_image_pixels_matches_documented_cap() {
        // The constant is part of the public API. A test
        // guards against an accidental change to its
        // value.
        assert_eq!(MAX_IMAGE_PIXELS, 64 * 1024 * 1024);
    }
}
