//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Bitmap format handler (`wxBitmapHandler`).

use std::path::Path;

use crate::dc::bitmap::Bitmap;
use crate::dc::image::Image;

/// Creates bitmaps from image files (`wxBitmapHandler`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitmapHandler;

impl Default for BitmapHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl BitmapHandler {
    pub const fn new() -> Self {
        Self
    }

    pub fn load(&self, path: impl AsRef<Path>) -> Option<Bitmap> {
        Image::load_from_file(path.as_ref())
            .ok()
            .map(|img| img.to_bitmap())
    }
}
