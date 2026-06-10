//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! SVG-backed bitmap (`wxSVGBitmap`).

use std::path::Path;

use crate::dc::bitmap::Bitmap;

/// Bitmap rasterised from SVG data (`wxSVGBitmap`).
#[derive(Debug, Clone)]
pub struct SVGBitmap {
    width: u32,
    height: u32,
    #[cfg(target_os = "windows")]
    bitmap: Option<Bitmap>,
}

impl SVGBitmap {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            #[cfg(target_os = "windows")]
            bitmap: None,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Load SVG bytes and rasterise at the configured size.
    pub fn load_from_bytes(&mut self, svg_bytes: &[u8]) -> bool {
        #[cfg(target_os = "windows")]
        {
            if let Some(hbmp) =
                crate::dc::icon::svg_bytes_to_hbitmap(svg_bytes, self.width, self.height)
            {
                self.bitmap = Some(unsafe { Bitmap::from_hbitmap(hbmp, self.width, self.height) });
                return true;
            }
            false
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = svg_bytes;
            false
        }
    }

    /// Load SVG from a file path.
    pub fn load_from_file(&mut self, path: impl AsRef<Path>) -> bool {
        std::fs::read(path)
            .ok()
            .is_some_and(|bytes| self.load_from_bytes(&bytes))
    }

    /// Borrow the underlying bitmap, if loaded.
    pub fn bitmap(&self) -> Option<&Bitmap> {
        #[cfg(target_os = "windows")]
        {
            self.bitmap.as_ref()
        }

        #[cfg(not(target_os = "windows"))]
        {
            None
        }
    }
}
