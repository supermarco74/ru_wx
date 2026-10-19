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
    ImageList_Add, ImageList_Create, ImageList_Destroy, ILC_COLOR32,
};

/// Raw handle to a Win32 `HIMAGELIST`. Exposed so that widgets can pass
/// it to messages such as `LVM_SETIMAGELIST`.
#[cfg(target_os = "windows")]
pub type ImageListHandle = isize;

/// Stub handle used on non-Windows platforms (always zero).
#[cfg(not(target_os = "windows"))]
pub type ImageListHandle = isize;

/// Payload condiviso della lista: il `HIMAGELIST` nativo viene
/// distrutto **solo** quando l'ultimo riferimento (`Arc`) viene
/// rilasciato. In questo modo un controllo che salva una copia
/// (`ImageList::clone`) mantiene viva la lista anche se il creatore
/// la fa cadere a fine scope — senza questo, l'handle memorizzato
/// da toolbar/notebook diventa dangling e le icone spariscono.
struct ImageListInner {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    handle: ImageListHandle,
    /// Cached size for introspection.
    width: i32,
    height: i32,
}

/// A collection of same-sized bitmaps that can be referenced by index.
///
/// Il tipo è **condiviso**: `clone()` non duplica la lista nativa ma
/// crea un nuovo riferimento allo stesso `HIMAGELIST` (conto `Arc`),
/// così più controlli possono attaccare la stessa lista in sicurezza.
#[derive(Clone)]
pub struct ImageList {
    inner: std::sync::Arc<ImageListInner>,
}

impl ImageList {
    /// Create a new image list. Each image added later will be drawn
    /// at the given `width` x `height` (in pixels). On Windows the
    /// image list uses 32-bit colour (`ILC_COLOR32`) so that icons with
    /// alpha are rendered correctly.
    #[cfg(target_os = "windows")]
    pub fn new(width: i32, height: i32) -> Self {
        // `ILC_COLOR32` — 32-bpp DIBs with alpha (no separate mask).
        //
        // NB: non aggiungere `ILC_HIGHQUALITYSCALE`. Se il processo non
        // ha attivo il manifest comctl32 v6, `ImageList_Create` con
        // quel flag restituisce NULL (ERRORE 6) → ogni `add_image_bytes`
        // fallisce e **tutte le icone dell'app spariscono** (toolbar,
        // tree e lista). Meglio perdere un po' di nitidezza in scaling
        // HiDPI che restare senza icone.
        let flags = ILC_COLOR32;
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let h = unsafe { ImageList_Create(width, height, flags, 8, 8) };
        ImageList {
            inner: std::sync::Arc::new(ImageListInner {
                handle: h as ImageListHandle,
                width,
                height,
            }),
        }
    }

    /// macOS: allocate an AppKit-backed image list.
    #[cfg(target_os = "macos")]
    pub fn new(width: i32, height: i32) -> Self {
        ImageList {
            inner: std::sync::Arc::new(ImageListInner {
                handle: crate::platform::appkit::image_list_create(width, height),
                width,
                height,
            }),
        }
    }

    /// Non-macOS non-Windows stub.
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    pub fn new(width: i32, height: i32) -> Self {
        let _ = (width, height);
        ImageList {
            inner: std::sync::Arc::new(ImageListInner { width, height }),
        }
    }

    /// Add an SVG (embedded bytes) rasterised at this list's size.
    #[cfg(target_os = "windows")]
    pub fn add_svg_bytes(&self, svg: &[u8]) -> Option<i32> {
        let hb = crate::dc::icon::svg_bytes_to_hbitmap(
            svg,
            self.inner.width as u32,
            self.inner.height as u32,
        )?;
        self.add_bitmap(hb as _)
    }

    /// Add PNG / JPEG / GIF / WebP / BMP / SVG bytes at this list's size.
    #[cfg(target_os = "windows")]
    pub fn add_image_bytes(&self, bytes: &[u8]) -> Option<i32> {
        let hb = crate::dc::icon::image_bytes_to_hbitmap(
            bytes,
            self.inner.width as u32,
            self.inner.height as u32,
        )?;
        self.add_bitmap(hb as _)
    }

    #[cfg(target_os = "macos")]
    pub fn add_svg_bytes(&self, svg: &[u8]) -> Option<i32> {
        crate::platform::appkit::image_list_add_svg(self.inner.handle, svg)
    }

    #[cfg(target_os = "macos")]
    pub fn add_image_bytes(&self, bytes: &[u8]) -> Option<i32> {
        crate::platform::appkit::image_list_add_image_bytes(self.inner.handle, bytes)
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    pub fn add_svg_bytes(&self, _svg: &[u8]) -> Option<i32> {
        None
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    pub fn add_image_bytes(&self, _bytes: &[u8]) -> Option<i32> {
        None
    }

    /// Add a bitmap handle ([`NativeHandle`](crate::platform::NativeHandle)) to the image list.
    ///
    /// Returns the zero-based index of the new image, or `None` if the
    /// underlying call failed.
    pub fn add_bitmap(&self, hbitmap: crate::platform::NativeHandle) -> Option<i32> {
        #[cfg(target_os = "windows")]
        {
            if hbitmap == 0 {
                return None;
            }
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let idx = unsafe {
                ImageList_Add(
                    self.inner.handle as _,
                    hbitmap as HBITMAP as _,
                    std::ptr::null_mut(),
                )
            };
            if idx >= 0 {
                Some(idx)
            } else {
                None
            }
        }
        #[cfg(target_os = "macos")]
        {
            if hbitmap == 0 {
                return None;
            }
            crate::platform::appkit::image_list_add_mac_bitmap(self.inner.handle, hbitmap)
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            let _ = hbitmap;
            Some(self.count())
        }
    }

    /// Return the raw `HIMAGELIST` handle so the parent control can
    /// attach it (e.g. via `LVM_SETIMAGELIST`).
    pub fn handle(&self) -> ImageListHandle {
        #[cfg(target_os = "windows")]
        {
            self.inner.handle
        }
        #[cfg(target_os = "macos")]
        {
            self.inner.handle
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            0
        }
    }

    /// Width of each image in the list, in pixels.
    pub fn width(&self) -> i32 {
        self.inner.width
    }
    /// Height of each image in the list, in pixels.
    pub fn height(&self) -> i32 {
        self.inner.height
    }

    /// Number of images currently stored.
    #[cfg(target_os = "windows")]
    pub fn count(&self) -> i32 {
        // SAFETY: `ImageList_GetImageCount` on a live `HIMAGELIST`.
        unsafe { windows_sys::Win32::UI::Controls::ImageList_GetImageCount(self.inner.handle as _) }
    }

    #[cfg(target_os = "macos")]
    pub fn count(&self) -> i32 {
        crate::platform::appkit::image_list_count(self.inner.handle)
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    pub fn count(&self) -> i32 {
        0
    }
}

#[cfg(target_os = "macos")]
impl Drop for ImageListInner {
    fn drop(&mut self) {
        if self.handle != 0 {
            crate::platform::appkit::image_list_destroy(self.handle);
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for ImageListInner {
    fn drop(&mut self) {
        if self.handle != 0 {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                ImageList_Destroy(self.handle as _);
            }
        }
    }
}
