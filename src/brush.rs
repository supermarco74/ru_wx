//! Fill brush (`wxBrush`).
//!
//! A [`Brush`] is a graphic object used by a
//! [`crate::dc::DC`] to fill shapes (rectangles, ellipses,
//! polygons). It pairs a colour with a fill style
//! (solid, transparent, hatched).
//!
//! # Win32 model
//!
//! On Windows a brush is an `HBRUSH` GDI handle. The
//! constructor uses `CreateSolidBrush` for solid fills and
//! `GetStockObject(NULL_BRUSH)` for the transparent case.
//! The handle is freed by [`Brush::destroy`] / [`Drop`].
//!
//! # Cross-platform stub
//!
//! On non-Windows targets [`Brush`] is a data-only struct
//! (no real drawing happens).

use crate::geometry::Colour;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{
    CreateSolidBrush, DeleteObject, GetStockObject, HBRUSH, NULL_BRUSH,
};

/// Fill style of a brush. Maps to the Win32 stock-object /
/// hatch constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BrushStyle {
    /// Solid colour fill.
    Solid,
    /// Transparent fill (shapes are not filled, only their
    /// outline is drawn).
    Transparent,
}

/// A draw-time brush.
#[derive(Debug, Clone)]
pub struct Brush {
    /// Fill colour.
    pub colour: Colour,
    /// Fill style.
    pub style: BrushStyle,
    /// Win32 GDI handle, populated by the constructor. `0`
    /// on non-Windows targets.
    #[cfg(target_os = "windows")]
    hbrush: HBRUSH,
    /// `true` if the handle was obtained via
    /// `GetStockObject` and must **not** be passed to
    /// `DeleteObject`. `false` for handles we own.
    #[cfg(target_os = "windows")]
    is_stock: bool,
}

impl Brush {
    /// Create a new brush of the given colour and style.
    pub fn new(colour: Colour, style: BrushStyle) -> Self {
        #[cfg(target_os = "windows")]
        let (hbrush, is_stock) = unsafe {
            match style {
                BrushStyle::Solid => (CreateSolidBrush(colour.to_colorref()), false),
                BrushStyle::Transparent => (GetStockObject(NULL_BRUSH), true),
            }
        };
        Self {
            colour,
            style,
            #[cfg(target_os = "windows")]
            hbrush,
            #[cfg(target_os = "windows")]
            is_stock,
        }
    }

    /// Create a solid-colour brush. Equivalent to
    /// `Brush::new(colour, BrushStyle::Solid)`.
    pub fn solid(colour: Colour) -> Self {
        Self::new(colour, BrushStyle::Solid)
    }

    /// Borrow the raw `HBRUSH` handle. Returns `0` on
    /// non-Windows targets. The handle is *borrowed* — do
    /// not call `DeleteObject` on it (and in particular
    /// **never** on a stock object), [`Brush::destroy`]
    /// owns the lifetime.
    #[cfg(target_os = "windows")]
    pub fn handle(&self) -> HBRUSH {
        self.hbrush
    }

    /// Returns `true` if this brush is a stock object and
    /// must not be deleted. The transparent stock brush
    /// is the only stock object we expose.
    #[cfg(target_os = "windows")]
    pub fn is_stock(&self) -> bool {
        self.is_stock
    }

    /// Free the underlying `HBRUSH`. Safe to call multiple
    /// times; subsequent calls are no-ops. Stock objects
    /// are *not* deleted. Always called automatically by
    /// [`Drop`].
    #[cfg(target_os = "windows")]
    pub fn destroy(&mut self) {
        if !self.hbrush.is_null() && !self.is_stock {
            // SAFETY: `hbrush` was created by
            // `CreateSolidBrush` in the constructor and
            // has not been destroyed. We only reach this
            // branch for owned handles, never for stock
            // objects.
            unsafe {
                DeleteObject(self.hbrush);
            }
        }
        self.hbrush = std::ptr::null_mut();
        self.is_stock = false;
    }
}

#[cfg(target_os = "windows")]
impl Drop for Brush {
    fn drop(&mut self) {
        self.destroy();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brush_solid_default() {
        let b = Brush::solid(Colour::BLACK);
        assert_eq!(b.colour, Colour::BLACK);
        assert_eq!(b.style, BrushStyle::Solid);
    }

    #[test]
    fn brush_transparent_records_style() {
        let b = Brush::new(Colour::WHITE, BrushStyle::Transparent);
        assert_eq!(b.colour, Colour::WHITE);
        assert_eq!(b.style, BrushStyle::Transparent);
    }

    #[test]
    fn brush_styles_distinct() {
        assert_ne!(BrushStyle::Solid, BrushStyle::Transparent);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn solid_brush_handle_is_nonnull_and_owned() {
        let b = Brush::solid(Colour::BLACK);
        assert!(!b.handle().is_null());
        assert!(!b.is_stock());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn transparent_brush_handle_is_nonnull_and_stock() {
        let b = Brush::new(Colour::WHITE, BrushStyle::Transparent);
        assert!(!b.handle().is_null());
        assert!(b.is_stock());
    }
}
