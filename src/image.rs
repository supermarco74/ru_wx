//! `wxImage` — image container with format-independent pixel access.
//!
//! An [`Image`] holds a decoded raster image in memory as a
//! packed RGBA8 buffer, together with its pixel dimensions. It
//! is the format-independent counterpart of [`crate::bitmap::Bitmap`]:
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
//! [`crate::dc::Dc`]. The pixel data is swizzled from RGBA
//! to BGRA (the layout Win32 expects for 32-bit DIBs).
//!
//! # Cross-platform stub
//!
//! On non-Windows targets the type is still usable — you can
//! load images and inspect their pixel data — but
//! [`Image::to_bitmap`] returns a stub `Bitmap` (no real
//! backing store) because there is no GDI.

use crate::bitmap::Bitmap;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{
    CreateDIBSection, GetDC, ReleaseDC, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS,
};

/// 8-bit RGBA pixel, stored as `(R, G, B, A)` in memory.
pub type Rgba = (u8, u8, u8, u8);

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

impl Image {
    /// Create an empty image of the given dimensions filled
    /// with opaque black. Mainly useful for tests and for
    /// building images pixel-by-pixel.
    pub fn new(width: u32, height: u32) -> Self {
        let pixels = vec![0u8; (width as usize) * (height as usize) * 4];
        Self {
            width,
            height,
            pixels,
        }
    }

    /// Create an image from a raw RGBA8 pixel buffer. The
    /// buffer length must be exactly `width * height * 4`
    /// bytes. Used by the GIF/APNG decoder in
    /// [`crate::animation::Animation`] and by anyone who needs
    /// to build an `Image` from existing RGBA data.
    pub fn from_rgba8(width: u32, height: u32, pixels: Vec<u8>) -> Self {
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
    /// coordinates are out of range.
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<Rgba> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
        Some((self.pixels[idx], self.pixels[idx + 1], self.pixels[idx + 2], self.pixels[idx + 3]))
    }

    /// Set the RGBA8 pixel at `(x, y)`. Returns `false` if
    /// the coordinates are out of range.
    pub fn set_pixel(&mut self, x: u32, y: u32, pixel: Rgba) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let idx = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
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
}
