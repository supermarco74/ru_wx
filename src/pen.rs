//! Drawing pen (`wxPen`).
//!
//! A [`Pen`] is a graphic object that is **not** a widget; it
//! describes how a [`crate::dc::DC`] renders strokes: a colour,
//! a width in pixels, and an optional line style (solid, dotted,
//! dashed, …).
//!
//! # Win32 model
//!
//! On Windows a pen is represented as an `HPEN` GDI handle. The
//! constructor creates a `PS_SOLID` cosmetic pen with a single
//! colour and a width in pixels. The `HPEN` is reference-counted
//! by the GDI kernel; we use `DeleteObject` in [`Pen::destroy`]
//! when the pen is no longer in use. The struct stores the
//! handle alongside the colour / width / style metadata so the
//! caller can introspect the pen without re-querying GDI.
//!
//! # Cross-platform stub
//!
//! On non-Windows targets [`Pen`] is a pure data struct that
//! carries the colour / width / style. Selecting it into a DC is
//! a no-op (drawing in the cross-platform target is not
//! implemented). The `*_from_*` Win32 functions are not
//! available off-Windows.

use crate::geometry::Colour;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{
    CreatePen, DeleteObject, HPEN, PS_DASH, PS_DOT, PS_NULL, PS_SOLID,
};

/// Style of a pen. Maps to the Win32 pen-style flags
/// (`PS_SOLID`, `PS_DOT`, `PS_DASH`, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PenStyle {
    /// Continuous solid line.
    Solid,
    /// Dotted line.
    Dot,
    /// Dashed line.
    Dash,
    /// No visible line (the pen is a "null" pen, like
    /// `wxPENSTYLE_TRANSPARENT`).
    Transparent,
}

#[cfg(target_os = "windows")]
fn pen_style_to_win32(style: PenStyle) -> i32 {
    // `PEN_STYLE` is `pub type PEN_STYLE = i32;` in
    // windows-sys 0.59, so the values can be used as `i32`
    // directly. PS_DOT / PS_DASH / PS_NULL are exposed as
    // `pub const` items in `Win32::Graphics::Gdi`.
    match style {
        PenStyle::Solid => PS_SOLID,
        PenStyle::Dot => PS_DOT,
        PenStyle::Dash => PS_DASH,
        PenStyle::Transparent => PS_NULL,
    }
}

/// A draw-time pen.
#[derive(Debug, Clone)]
pub struct Pen {
    /// RGBA pen colour. The alpha channel is ignored by Win32
    /// GDI pens — Windows pens are always opaque.
    pub colour: Colour,
    /// Pen width in pixels. `1` is the standard hairline.
    pub width: u32,
    /// Stroke style.
    pub style: PenStyle,
    /// Win32 GDI handle, populated by the constructor. `0`
    /// on non-Windows targets.
    #[cfg(target_os = "windows")]
    hpen: HPEN,
}

impl Pen {
    /// Create a new pen with the given colour, width and
    /// style. The default style is [`PenStyle::Solid`].
    ///
    /// On Windows this builds the `HPEN` immediately; on
    /// other targets the Win32 handle is left as `0`.
    pub fn new(colour: Colour, width: u32, style: PenStyle) -> Self {
        #[cfg(target_os = "windows")]
        let hpen = unsafe { CreatePen(pen_style_to_win32(style), width as i32, colour.to_colorref()) };
        Self {
            colour,
            width,
            style,
            #[cfg(target_os = "windows")]
            hpen,
        }
    }

    /// Create a 1-pixel-wide solid pen of the given colour.
    /// The most common pen — equivalent to
    /// `Pen::new(colour, 1, PenStyle::Solid)`.
    pub fn solid(colour: Colour) -> Self {
        Self::new(colour, 1, PenStyle::Solid)
    }

    /// Borrow the raw `HPEN` handle. Returns `0` on
    /// non-Windows targets. The handle is *borrowed* — do
    /// **not** call `DeleteObject` on it, [`Pen::destroy`]
    /// owns the lifetime.
    #[cfg(target_os = "windows")]
    pub fn handle(&self) -> HPEN {
        self.hpen
    }

    /// Free the underlying `HPEN`. Safe to call multiple
    /// times; subsequent calls are no-ops. Always called
    /// automatically by [`Drop`].
    #[cfg(target_os = "windows")]
    pub fn destroy(&mut self) {
        if !self.hpen.is_null() {
            // SAFETY: `hpen` was created by `CreatePen` in
            // the constructor and has not been destroyed
            // yet. The handle is owned by this `Pen` and
            // no other code path can call `DeleteObject`
            // on it.
            unsafe {
                DeleteObject(self.hpen);
            }
            self.hpen = std::ptr::null_mut();
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for Pen {
    fn drop(&mut self) {
        self.destroy();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pen_solid_default() {
        let p = Pen::solid(Colour::BLACK);
        assert_eq!(p.colour, Colour::BLACK);
        assert_eq!(p.width, 1);
        assert_eq!(p.style, PenStyle::Solid);
    }

    #[test]
    fn pen_new_preserves_fields() {
        let p = Pen::new(Colour::WHITE, 3, PenStyle::Dash);
        assert_eq!(p.colour, Colour::WHITE);
        assert_eq!(p.width, 3);
        assert_eq!(p.style, PenStyle::Dash);
    }

    #[test]
    fn pen_styles_distinct() {
        assert_ne!(PenStyle::Solid, PenStyle::Dot);
        assert_ne!(PenStyle::Dash, PenStyle::Transparent);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn pen_handle_is_nonnull_after_new() {
        let p = Pen::solid(Colour::BLACK);
        assert!(!p.handle().is_null());
    }
}
