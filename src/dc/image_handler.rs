//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Image format handler (`wxImageHandler`).

use std::path::Path;

use crate::dc::image::{Image, ImageError};

/// Loads images by extension (`wxImageHandler`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageHandler {
    pub extension: &'static str,
}

impl ImageHandler {
    pub const fn new(extension: &'static str) -> Self {
        Self { extension }
    }

    pub fn can_load(&self, path: impl AsRef<Path>) -> bool {
        path.as_ref()
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case(self.extension))
    }

    pub fn load(&self, path: impl AsRef<Path>) -> Result<Image, ImageError> {
        Image::load_from_file(path.as_ref())
    }
}
