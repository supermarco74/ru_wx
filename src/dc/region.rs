//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Clipping region (`wxRegion`).

use crate::core::geometry::Rect;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{CreateRectRgn, DeleteObject, HRGN};

/// A rectangular GDI region.
pub struct Region {
    #[cfg(target_os = "windows")]
    hrgn: HRGN,
}

impl Region {
    /// Rectangle region from `rect`.
    #[cfg(target_os = "windows")]
    pub fn from_rect(rect: Rect) -> Self {
        let hrgn = unsafe {
            CreateRectRgn(
                rect.x,
                rect.y,
                rect.x + rect.width as i32,
                rect.y + rect.height as i32,
            )
        };
        Self { hrgn }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn from_rect(_rect: Rect) -> Self {
        Self
    }

    #[cfg(target_os = "windows")]
    pub fn handle(&self) -> HRGN {
        self.hrgn
    }
}

#[cfg(target_os = "windows")]
impl Drop for Region {
    fn drop(&mut self) {
        if !self.hrgn.is_null() {
            unsafe {
                DeleteObject(self.hrgn as _);
            }
        }
    }
}
