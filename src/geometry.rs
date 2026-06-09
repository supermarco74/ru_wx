//! Geometric primitives shared across the widget layer.
//!
//! [`Rect`] is the position-and-size rectangle used by widget layout
//! code. [`Colour`] is a simple RGBA value; the [`Colour::to_colorref`]
//! helper converts to the `0x00BBGGRR` form expected by the Win32
//! `COLORREF` API.

/// Rectangle with position and size
#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x
            && px < self.x + self.width as i32
            && py >= self.y
            && py < self.y + self.height as i32
    }
}

/// RGBA colour
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Colour {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const WHITE: Self = Self::new(255, 255, 255, 255);
    pub const BLACK: Self = Self::new(0, 0, 0, 255);
    pub const LIGHT_GREY: Self = Self::new(240, 240, 240, 255);

    /// Convert to Win32 COLORREF format (0x00BBGGRR)
    #[cfg(target_os = "windows")]
    pub fn to_colorref(self) -> u32 {
        (self.b as u32) << 16 | (self.g as u32) << 8 | (self.r as u32)
    }
}

impl Default for Colour {
    fn default() -> Self {
        Self::WHITE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_new_keeps_fields() {
        let r = Rect::new(10, 20, 30, 40);
        assert_eq!(r.x, 10);
        assert_eq!(r.y, 20);
        assert_eq!(r.width, 30);
        assert_eq!(r.height, 40);
    }

    #[test]
    fn rect_default_is_origin_zero_zero() {
        let r = Rect::default();
        assert_eq!(r.x, 0);
        assert_eq!(r.y, 0);
        assert_eq!(r.width, 0);
        assert_eq!(r.height, 0);
    }

    #[test]
    fn rect_contains_is_inclusive_min_exclusive_max() {
        let r = Rect::new(10, 20, 30, 40); // 10..40 x 20..60
                                           // strict lower-left corner is contained
        assert!(r.contains(10, 20));
        // strict upper-right corner is NOT contained (exclusive)
        assert!(!r.contains(40, 60));
        // inside
        assert!(r.contains(25, 40));
        // outside
        assert!(!r.contains(9, 20));
        assert!(!r.contains(10, 19));
        assert!(!r.contains(100, 100));
    }

    #[test]
    fn colour_constants_have_expected_channels() {
        assert_eq!(Colour::WHITE, Colour::new(255, 255, 255, 255));
        assert_eq!(Colour::BLACK, Colour::new(0, 0, 0, 255));
        assert_eq!(Colour::LIGHT_GREY, Colour::new(240, 240, 240, 255));
    }

    #[test]
    fn colour_default_is_white() {
        assert_eq!(Colour::default(), Colour::WHITE);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn colour_to_colorref_is_bbggrr() {
        // Pure red:    0x000000FF
        // Pure green:  0x0000FF00
        // Pure blue:   0x00FF0000
        // Mid grey:    0x00808080
        assert_eq!(
            Colour::new(0xFF, 0x00, 0x00, 0x00).to_colorref(),
            0x0000_00FF
        );
        assert_eq!(
            Colour::new(0x00, 0xFF, 0x00, 0x00).to_colorref(),
            0x0000_FF00
        );
        assert_eq!(
            Colour::new(0x00, 0x00, 0xFF, 0x00).to_colorref(),
            0x00FF_0000
        );
        assert_eq!(
            Colour::new(0x80, 0x80, 0x80, 0x00).to_colorref(),
            0x0080_8080
        );
    }
}
