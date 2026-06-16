//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! `wxBitmapBundle` — a multi-resolution bitmap for HiDPI support.
//!
//! A `BitmapBundle` is a single logical bitmap that can hold several
//! rasterisations (typically 1×, 1.5× and 2×) of the same image. When a
//! control needs to draw the bitmap, it picks the most appropriate
//! resolution for the current DPI.
//!
//! On Windows there is no built-in multi-resolution HBITMAP structure
//! (one would normally use a `.ico` file or `HICON` for that), so this
//! type is simply a thin owner of multiple [`RawBitmap`]s. The
//! `best_for_size` and `best_for_dpi` helpers let callers pick the
//! right entry.
//!
//! Use [`BitmapBundle::from_svg_bytes`] to render an SVG at several
//! sizes in one go (the easiest way to get a HiDPI-aware icon from
//! a vector source).
//!
//! Use [`BitmapBundle::from_svg_path`] for the same thing, reading the
//! SVG from disk.
//!
//! Use [`BitmapBundle::from_bitmap`] for a single-resolution bundle.

#[cfg(target_os = "windows")]
use crate::dc::icon::svg_bytes_to_hbitmap;
use std::path::Path;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{DeleteObject, GetDC, ReleaseDC, HBITMAP};

/// A single rasterised entry in a [`BitmapBundle`].
#[cfg(target_os = "windows")]
#[derive(Clone, Copy)]
pub struct RawBitmap {
    /// Native handle. On Windows this is an `HBITMAP`. The bundle
    /// owns the handle and will delete it on `Drop`.
    pub hbitmap: HBITMAP,
    /// Pixel width of this rasterisation.
    pub width: u32,
    /// Pixel height of this rasterisation.
    pub height: u32,
}

#[cfg(not(target_os = "windows"))]
#[derive(Clone, Copy)]
pub struct RawBitmap {
    pub width: u32,
    pub height: u32,
}

/// Multi-resolution bitmap. Holds zero or more [`RawBitmap`]s, each at
/// a different pixel size.
pub struct BitmapBundle {
    #[cfg(target_os = "windows")]
    bitmaps: Vec<RawBitmap>,
    /// The "logical" size (the size at 1× DPI). When a control asks
    /// for a bundle of size 16×16, it gets the entry that best matches
    /// (width = 16, height = 16) at the current DPI.
    logical_size: (u32, u32),
}

impl BitmapBundle {
    /// Create an empty bundle. Bundle methods that require at least
    /// one entry will return a "missing" handle in this case.
    pub fn new() -> Self {
        BitmapBundle {
            #[cfg(target_os = "windows")]
            bitmaps: Vec::new(),
            logical_size: (0, 0),
        }
    }

    /// Create a single-resolution bundle from a single [`RawBitmap`].
    pub fn from_raw_bitmap(bmp: RawBitmap) -> Self {
        let mut bundle = BitmapBundle::new();
        bundle.logical_size = (bmp.width, bmp.height);
        #[cfg(target_os = "windows")]
        bundle.bitmaps.push(bmp);
        bundle
    }

    /// Create a single-resolution bundle from an already-existing
    /// `HBITMAP`.
    #[cfg(target_os = "windows")]
    pub fn from_bitmap(hbitmap: HBITMAP, width: u32, height: u32) -> Self {
        Self::from_raw_bitmap(RawBitmap {
            hbitmap,
            width,
            height,
        })
    }

    /// Add a rasterisation to the bundle.
    #[cfg(target_os = "windows")]
    pub fn add(&mut self, bmp: RawBitmap) {
        if self.logical_size == (0, 0) {
            self.logical_size = (bmp.width, bmp.height);
        }
        self.bitmaps.push(bmp);
    }

    /// Non-Windows stub.
    #[cfg(not(target_os = "windows"))]
    pub fn add(&mut self, bmp: RawBitmap) {
        if self.logical_size == (0, 0) {
            self.logical_size = (bmp.width, bmp.height);
        }
    }

    /// Create a multi-resolution bundle by rendering the given SVG
    /// bytes at the given list of pixel sizes.
    ///
    /// Common usage: `BitmapBundle::from_svg_bytes(svg, &[(16, 16),
    /// (24, 24), (32, 32)])` to get 1×/1.5×/2× variants of a 16-px
    /// icon.
    #[cfg(target_os = "windows")]
    pub fn from_svg_bytes(svg_bytes: &[u8], sizes: &[(u32, u32)]) -> Self {
        let mut bundle = BitmapBundle::new();
        for &(w, h) in sizes {
            if let Some(hbmp) = svg_bytes_to_hbitmap(svg_bytes, w, h) {
                bundle.add(RawBitmap {
                    hbitmap: hbmp,
                    width: w,
                    height: h,
                });
            }
        }
        bundle
    }

    /// Non-Windows stub.
    #[cfg(not(target_os = "windows"))]
    pub fn from_svg_bytes(_svg_bytes: &[u8], sizes: &[(u32, u32)]) -> Self {
        let mut bundle = BitmapBundle::new();
        if let Some(&(w, h)) = sizes.first() {
            bundle.logical_size = (w, h);
        }
        bundle
    }

    /// Create a multi-resolution bundle by reading the SVG at the
    /// given path and rendering it at the given sizes.
    #[cfg(target_os = "windows")]
    pub fn from_svg_path(path: &Path, sizes: &[(u32, u32)]) -> Option<Self> {
        let data = std::fs::read(path).ok()?;
        Some(Self::from_svg_bytes(&data, sizes))
    }

    /// Non-Windows stub.
    #[cfg(not(target_os = "windows"))]
    pub fn from_svg_path(_path: &Path, sizes: &[(u32, u32)]) -> Option<Self> {
        Some(Self::from_svg_bytes(&[], sizes))
    }

    /// Pick the entry whose size best matches `target_size` (in
    /// pixels) and return it. Returns `None` if the bundle is empty.
    pub fn best_for_size(&self, target_size: (u32, u32)) -> Option<RawBitmap> {
        #[cfg(target_os = "windows")]
        {
            self.bitmaps
                .iter()
                .min_by_key(|bmp| {
                    let dw = (bmp.width as i64 - target_size.0 as i64).abs();
                    let dh = (bmp.height as i64 - target_size.1 as i64).abs();
                    dw + dh
                })
                .copied()
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = target_size;
            None
        }
    }

    /// Pick the entry whose size best matches the current DPI. `dpi`
    /// is the dots-per-inch of the target device (96 for normal, 192
    /// for 200% scaled, etc.).
    #[cfg(target_os = "windows")]
    pub fn best_for_dpi(&self, dpi: u32) -> Option<RawBitmap> {
        // The "ideal" size for a 1× icon at `dpi` is `dpi / 96` pixels.
        // For example, at 192 DPI a 16-px icon should be drawn as a
        // 32-px bitmap.
        let scale = dpi as f32 / 96.0;
        let target_w = (self.logical_size.0 as f32 * scale).round() as u32;
        let target_h = (self.logical_size.1 as f32 * scale).round() as u32;
        self.best_for_size((target_w, target_h))
    }

    /// Non-Windows stub.
    #[cfg(not(target_os = "windows"))]
    pub fn best_for_dpi(&self, _dpi: u32) -> Option<RawBitmap> {
        None
    }

    /// Return the entry that the system considers a good match for
    /// drawing the bundle into a window with the given `HWND`. We use
    /// the `GetDC` / `GetDeviceCaps(LOGPIXELSX)` pair to query the
    /// current DPI.
    #[cfg(target_os = "windows")]
    #[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper, no pointer deref in user code
    pub fn best_for_hwnd(&self, hwnd: HWND) -> Option<RawBitmap> {
        // SAFETY: `GetDC` / `ReleaseDC` accept any `HWND` (including
        // null) and never dereference the handle in user code; Win32
        // does. `hdc` is checked for null before use.
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let hdc = GetDC(hwnd);
            if hdc.is_null() {
                return self.best_for_dpi(96);
            }
            let dpi_x = crate::platform::win32::get_device_caps_dpi(hdc);
            ReleaseDC(hwnd, hdc);
            self.best_for_dpi(dpi_x)
        }
    }

    /// Non-Windows stub.
    #[cfg(not(target_os = "windows"))]
    pub fn best_for_hwnd(&self, _hwnd: isize) -> Option<RawBitmap> {
        None
    }

    /// Logical (1× DPI) size of the bundle.
    pub fn logical_size(&self) -> (u32, u32) {
        self.logical_size
    }

    /// Number of rasterisations in the bundle.
    pub fn len(&self) -> usize {
        #[cfg(target_os = "windows")]
        {
            self.bitmaps.len()
        }
        #[cfg(not(target_os = "windows"))]
        {
            0
        }
    }

    /// `true` if the bundle has no rasterisations.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The HBITMAP of the first entry (or 0 on non-Windows / empty).
    pub fn handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.bitmaps
                .first()
                .map(|b| b.hbitmap as isize)
                .unwrap_or(0)
        }
        #[cfg(not(target_os = "windows"))]
        {
            0
        }
    }
}

impl Default for BitmapBundle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "windows")]
impl Drop for BitmapBundle {
    fn drop(&mut self) {
        for bmp in &self.bitmaps {
            if !bmp.hbitmap.is_null() {
                // SAFETY: FFI call to DeleteObject on GDI handles we own.
                unsafe {
                    DeleteObject(bmp.hbitmap);
                }
            }
        }
    }
}
