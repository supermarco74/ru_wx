//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Shared graphic tokens (`PIANO_PORTABILITA_COMPONENTI.md` §5).
//!
//! Single source of truth for spacing, radius, control height, font
//! sizes and the system accent colour. Every backend (Win32,
//! WinUI3/XAML, AppKit) consumes these constants; no colour or
//! spacing literal should be written by hand in individual controls.
//!
//! | Token | Value |
//! |---|---|
//! | spacing xs/s/m/l | 4 / 8 / 12 / 16 px |
//! | radius s / m | 4 / 8 px |
//! | control height | 32 px (28 px compat) |
//! | font | 13 pt text, 12 pt caption |
//! | accent | system accent ([`system_accent_colour`]) |
//!
//! Corner radii are consumed by the XAML backend (`CornerRadius`);
//! the Win32 backend paints rounded corners through the DWM window
//! corner preference (`Frame::set_corner_preference`) and therefore
//! does not use [`RADIUS_S`] / [`RADIUS_M`] directly.

use crate::core::geometry::Colour;

// ── Spacing (px) ───────────────────────────────────────────────────────

/// Spacing xs — 4 px: tight inline gaps (icon ↔ label).
pub const SPACING_XS: i32 = 4;
/// Spacing s — 8 px: gap between adjacent related controls.
pub const SPACING_S: i32 = 8;
/// Spacing m — 12 px: padding inside a group.
pub const SPACING_M: i32 = 12;
/// Spacing l — 16 px: padding between sections.
pub const SPACING_L: i32 = 16;

// ── Corner radius (px) ─────────────────────────────────────────────────

/// Radius s — 4 px: standard controls.
pub const RADIUS_S: i32 = 4;
/// Radius m — 8 px: cards and panels.
pub const RADIUS_M: i32 = 8;

// ── Control metrics (px) ───────────────────────────────────────────────

/// Standard control height — 32 px.
pub const CONTROL_HEIGHT: i32 = 32;
/// Compatibility control height — 28 px, for dense layouts and
/// legacy dialogs that predate the 32 px standard.
pub const CONTROL_HEIGHT_COMPAT: i32 = 28;

// ── Font sizes (pt) ────────────────────────────────────────────────────

/// Body text — 13 pt.
pub const FONT_SIZE: i32 = 13;
/// Caption text — 12 pt.
pub const FONT_SIZE_CAPTION: i32 = 12;

// ── Accent ─────────────────────────────────────────────────────────────

/// Fallback accent (#0078D7, the classic Windows blue) used when the
/// OS does not expose an accent colour or the query fails.
pub const ACCENT_FALLBACK: Colour = Colour::new(0, 120, 215, 255);

/// The user-chosen system accent colour (§5 "Accent di sistema").
///
/// On Windows this reads `DwmGetColorizationColor`, which returns
/// 0xAARRGGBB (not a COLORREF); the alpha byte is dropped because
/// the token is an opaque RGB value. On other targets the fallback
/// is returned — macOS should eventually answer with
/// `NSColor.controlAccentColor`.
pub fn system_accent_colour() -> Colour {
    system_accent_inner()
}

#[cfg(target_os = "windows")]
fn system_accent_inner() -> Colour {
    let mut argb: u32 = 0;
    let mut opaque_blend: i32 = 0;
    // SAFETY: both pointers reference live stack locals that the DWM
    // fills exactly once on success; the HRESULT tells us whether
    // they hold valid data.
    let hr = unsafe {
        windows_sys::Win32::Graphics::Dwm::DwmGetColorizationColor(&mut argb, &mut opaque_blend)
    };
    if hr < 0 {
        return ACCENT_FALLBACK;
    }
    // 0xAARRGGBB → RGB opaco (alpha ignorato, vedi doc).
    Colour::new(
        ((argb >> 16) & 0xFF) as u8,
        ((argb >> 8) & 0xFF) as u8,
        (argb & 0xFF) as u8,
        255,
    )
}

#[cfg(not(target_os = "windows"))]
fn system_accent_inner() -> Colour {
    ACCENT_FALLBACK
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pin the token values to the §5 table of
    /// `PIANO_PORTABILITA_COMPONENTI.md`: changing one of them here
    /// without updating the plan is a deviation, not a tweak.
    #[test]
    fn token_values_match_plan_section_5() {
        assert_eq!(
            (SPACING_XS, SPACING_S, SPACING_M, SPACING_L),
            (4, 8, 12, 16)
        );
        assert_eq!((RADIUS_S, RADIUS_M), (4, 8));
        assert_eq!((CONTROL_HEIGHT, CONTROL_HEIGHT_COMPAT), (32, 28));
        assert_eq!((FONT_SIZE, FONT_SIZE_CAPTION), (13, 12));
    }

    /// The accent token is an opaque RGB colour on every platform.
    #[test]
    fn accent_is_opaque() {
        assert_eq!(system_accent_colour().a, 255);
    }

    /// The fallback accent is the canonical #0078D7.
    #[test]
    fn fallback_accent_is_windows_blue() {
        assert_eq!(
            (
                ACCENT_FALLBACK.r,
                ACCENT_FALLBACK.g,
                ACCENT_FALLBACK.b,
                ACCENT_FALLBACK.a
            ),
            (0, 120, 215, 255)
        );
    }
}
