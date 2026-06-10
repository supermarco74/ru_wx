//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! `ImageList` — a wrapper around the Win32 `HIMAGELIST`.
//!
//! An `ImageList` is a collection of same-sized images that can be
//! referenced by index. List/Tree controls in report mode use image
//! lists to render small icons next to text in cells (or on their own).
//!
//! This module is the building block for the [`crate::Grid`] widget's
//! "cells with images" support. Other widgets that need icons (e.g. a
//! future bitmap-button or toolbar) can also use it.

#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::HBITMAP;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Controls::{
    ImageList_Add, ImageList_Create, ImageList_Destroy, ILC_COLOR32, ILC_HIGHQUALITYSCALE,
};

/// Raw handle to a Win32 `HIMAGELIST`. Exposed so that widgets can pass
/// it to messages such as `LVM_SETIMAGELIST`.
#[cfg(target_os = "windows")]
pub type ImageListHandle = isize;

/// Stub handle used on non-Windows platforms (always zero).
#[cfg(not(target_os = "windows"))]
pub type ImageListHandle = isize;

/// A collection of same-sized bitmaps that can be referenced by index.
pub struct ImageList {
    #[cfg(target_os = "windows")]
    handle: ImageListHandle,
    /// Cached size for introspection.
    width: i32,
    height: i32,
}

impl ImageList {
    /// Create a new image list. Each image added later will be drawn
    /// at the given `width` x `height` (in pixels). On Windows the
    /// image list uses 32-bit colour (`ILC_COLOR32`) so that icons with
    /// alpha are rendered correctly.
    #[cfg(target_os = "windows")]
    pub fn new(width: i32, height: i32) -> Self {
        // `ILC_COLOR32` — 32-bpp DIBs with alpha (no separate mask).
        // `ILC_HIGHQUALITYSCALE` — crisp scaling on HiDPI displays.
        let flags = ILC_COLOR32 | ILC_HIGHQUALITYSCALE;
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let h = unsafe { ImageList_Create(width, height, flags, 8, 8) };
        ImageList {
            handle: h as ImageListHandle,
            width,
            height,
        }
    }

    /// Non-Windows stub.
    #[cfg(not(target_os = "windows"))]
    pub fn new(width: i32, height: i32) -> Self {
        let _ = (width, height);
        ImageList { width, height }
    }

    /// Add an SVG (embedded bytes) rasterised at this list's size.
    #[cfg(target_os = "windows")]
    pub fn add_svg_bytes(&self, svg: &[u8]) -> Option<i32> {
        let hb = crate::dc::icon::svg_bytes_to_hbitmap(
            svg,
            self.width as u32,
            self.height as u32,
        )?;
        self.add_bitmap(hb)
    }

    /// Add PNG / JPEG / GIF / WebP / BMP / SVG bytes at this list's size.
    #[cfg(target_os = "windows")]
    pub fn add_image_bytes(&self, bytes: &[u8]) -> Option<i32> {
        let hb = crate::dc::icon::image_bytes_to_hbitmap(
            bytes,
            self.width as u32,
            self.height as u32,
        )?;
        self.add_bitmap(hb)
    }

    #[cfg(not(target_os = "windows"))]
    pub fn add_svg_bytes(&self, _svg: &[u8]) -> Option<i32> {
        None
    }

    #[cfg(not(target_os = "windows"))]
    pub fn add_image_bytes(&self, _bytes: &[u8]) -> Option<i32> {
        None
    }

    /// Add a bitmap (returned from [`crate::dc::icon::load_svg_as_hbitmap`]
    /// or any other 32-bpp DIB) to the image list.
    ///
    /// Returns the zero-based index of the new image, or `None` if the
    /// underlying call failed.
    #[cfg(target_os = "windows")]
    pub fn add_bitmap(&self, hbitmap: HBITMAP) -> Option<i32> {
        if hbitmap.is_null() {
            return None;
        }
        // Third argument is the monochrome mask bitmap — we pass null
        // because the 32-bpp DIBs we add already carry their own alpha
        // channel.
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let idx = unsafe { ImageList_Add(self.handle as _, hbitmap as _, std::ptr::null_mut()) };
        if idx >= 0 {
            Some(idx)
        } else {
            None
        }
    }

    /// Non-Windows stub.
    #[cfg(not(target_os = "windows"))]
    pub fn add_bitmap(&self, _hbitmap: ()) -> Option<i32> {
        None
    }

    /// Return the raw `HIMAGELIST` handle so the parent control can
    /// attach it (e.g. via `LVM_SETIMAGELIST`).
    pub fn handle(&self) -> ImageListHandle {
        #[cfg(target_os = "windows")]
        {
            self.handle
        }
        #[cfg(not(target_os = "windows"))]
        {
            0
        }
    }

    /// Width of each image in the list, in pixels.
    pub fn width(&self) -> i32 {
        self.width
    }
    /// Height of each image in the list, in pixels.
    pub fn height(&self) -> i32 {
        self.height
    }

    /// Number of images currently stored.
    #[cfg(target_os = "windows")]
    pub fn count(&self) -> i32 {
        // SAFETY: `ImageList_GetImageCount` on a live `HIMAGELIST`.
        unsafe {
            windows_sys::Win32::UI::Controls::ImageList_GetImageCount(self.handle as _)
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn count(&self) -> i32 {
        0
    }
}

#[cfg(target_os = "windows")]
impl Drop for ImageList {
    fn drop(&mut self) {
        if self.handle != 0 {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                ImageList_Destroy(self.handle as _);
            }
        }
    }
}
