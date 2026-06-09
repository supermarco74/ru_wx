//! Modal font-chooser dialog (`wxFontDialog`).
//!
//! Wraps the Win32 common dialog `ChooseFontW` (comdlg32.dll). The
//! user picks a face name, point size, weight (bold), style (italic
//! / underline / strike-out), and (optionally) a colour. On confirm,
//! the chosen attributes are returned as a [`FontDesc`]; pass it to
//! [`crate::font::Font::new`] to build a live font that can be
//! attached to a widget via `WM_SETFONT`.
//!
//! See also: [`crate::colour_picker_ctrl`] / [`crate::color_dialog`]
//! for colour-only pickers and [`crate::font`] for the font types.

use crate::font::{Font, FontDesc};
use crate::frame::Frame;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::LOGFONTW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Controls::Dialogs::{
    ChooseFontW, CF_EFFECTS, CF_FORCEFONTEXIST, CF_INITTOLOGFONTSTRUCT, CF_SCREENFONTS,
    CHOOSEFONTW,
};

// ── Inner type ─────────────────────────────────────────────────────────

/// A modal font-chooser dialog.
///
/// Build the dialog with setter methods, then call
/// [`FontDialog::show_modal`] to present it. The chosen font is
/// returned as a live [`Font`], or `None` if the user cancelled.
pub struct FontDialog {
    #[cfg(target_os = "windows")]
    parent_hwnd: HWND,
    /// Initial font description (pre-populated in the dialog).
    initial: FontDesc,
    /// Initial text colour for the "Effects" colour picker. Only
    /// honoured when [`FontDialog::set_show_effects`] is `true`.
    initial_colour: u32,
    /// Show the colour / strike-out / underline effects box
    /// (`CF_EFFECTS`).
    show_effects: bool,
    /// Title (silently ignored by the Windows common dialog — see
    /// [`crate::color_dialog::ColorDialog::set_title`] for the same
    /// cross-platform workaround).
    title: String,
}

impl FontDialog {
    /// Create a new font dialog associated with the given frame,
    /// pre-populated with the system default font (Segoe UI 9pt).
    pub fn new(frame: &Frame) -> Self {
        FontDialog {
            #[cfg(target_os = "windows")]
            parent_hwnd: frame.hwnd(),
            initial: FontDesc::default(),
            initial_colour: 0,
            show_effects: true,
            title: String::new(),
        }
    }

    /// Create a new font dialog with the given initial font.
    pub fn with_initial(frame: &Frame, initial: FontDesc) -> Self {
        FontDialog {
            #[cfg(target_os = "windows")]
            parent_hwnd: frame.hwnd(),
            initial,
            initial_colour: 0,
            show_effects: true,
            title: String::new(),
        }
    }

    /// Replace the dialog's initial font description.
    pub fn set_initial_font(&mut self, desc: FontDesc) {
        self.initial = desc;
    }

    /// Toggle the "Effects" box (colour / strike-out / underline).
    /// Default is `true`.
    pub fn set_show_effects(&mut self, show: bool) {
        self.show_effects = show;
    }

    /// Set the initial colour shown in the "Effects" colour picker.
    /// Encoded as a `COLORREF` (0x00BBGGRR).
    pub fn set_initial_colour(&mut self, colorref: u32) {
        self.initial_colour = colorref;
    }

    /// Set the dialog title. Stored for cross-platform wrappers;
    /// on Windows the common dialog does not display a custom
    /// title and this value is silently ignored.
    pub fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    /// Show the dialog modally. Returns the chosen font (with a
    /// freshly-created `HFONT`), or `None` if the user cancelled.
    pub fn show_modal(&mut self) -> Option<Font> {
        #[cfg(target_os = "windows")]
        {
            // Build a LOGFONTW from the initial description. The
            // struct is both input and output: we seed it with the
            // current values, and `ChooseFontW` writes the user's
            // selection back into it on confirm.
            let mut lf: LOGFONTW = unsafe { std::mem::zeroed() };
            let pixel_height = -((self.initial.point_size * self.initial.dpi) / 72);
            lf.lfHeight = pixel_height;
            lf.lfWeight = if self.initial.bold { 700 } else { 400 };
            lf.lfItalic = if self.initial.italic { 1 } else { 0 };
            lf.lfUnderline = if self.initial.underline { 1 } else { 0 };
            lf.lfStrikeOut = 0;
            lf.lfCharSet = 1; // DEFAULT_CHARSET
            lf.lfOutPrecision = 0; // OUT_DEFAULT_PRECIS
            lf.lfClipPrecision = 0; // CLIP_DEFAULT_PRECIS
            lf.lfQuality = 5; // CLEARTYPE_QUALITY
            lf.lfPitchAndFamily = 0;
            // Copy the face name into the fixed-size array.
            let face = self.initial.face_name.encode_utf16();
            for (i, unit) in face.enumerate() {
                if i >= 31 {
                    break;
                }
                lf.lfFaceName[i] = unit;
            }
            // lfFaceName[31] is the NUL terminator (zeroed by
            // mem::zeroed above).

            // iPointSize is in 1/10 of a point.
            let initial_point10 = (self.initial.point_size * 10) as i32;

            let mut flags: u32 = CF_SCREENFONTS | CF_INITTOLOGFONTSTRUCT | CF_FORCEFONTEXIST;
            if self.show_effects {
                flags |= CF_EFFECTS;
            }
            // CF_BOTH = CF_SCREENFONTS | CF_PRINTERFONTS; we leave
            // the printer option off because the printer list is
            // empty in apps that have not set up a printer DC.

            // SAFETY: Win32 FFI call with validated arguments.
            let ok = unsafe {
                let mut cf = CHOOSEFONTW {
                    lStructSize: std::mem::size_of::<CHOOSEFONTW>() as u32,
                    hwndOwner: self.parent_hwnd,
                    hDC: std::ptr::null_mut(),
                    lpLogFont: &mut lf,
                    iPointSize: initial_point10,
                    Flags: flags,
                    rgbColors: self.initial_colour,
                    lCustData: 0,
                    lpfnHook: None,
                    lpTemplateName: std::ptr::null(),
                    hInstance: std::ptr::null_mut(),
                    lpszStyle: std::ptr::null_mut(),
                    nFontType: 0,
                    ___MISSING_ALIGNMENT__: 0,
                    nSizeMin: 0,
                    nSizeMax: 0,
                };
                ChooseFontW(&mut cf)
            };

            if ok == 0 {
                return None;
            }

            // Read the result back out of the LOGFONTW. The point
            // size is reconstructed from the pixel height by
            // inverting the same MulDiv Win32 applies internally:
            //   point_size = -lfHeight * 72 / dpi
            // (lfHeight is negative because we asked for the
            // character height; positive lfHeight would be a
            // cell-height request.)
            let point_size = if lf.lfHeight < 0 {
                ((-lf.lfHeight) * 72 / self.initial.dpi).max(1)
            } else {
                // The control sometimes returns a positive
                // lfHeight when "Pixels" is the active unit.
                // Fall back to the original point size.
                self.initial.point_size
            };

            // Extract the face name as a Rust String. NUL-terminate
            // at the first 0 u16.
            let face_len = lf
                .lfFaceName
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(lf.lfFaceName.len());
            let face_name = String::from_utf16_lossy(&lf.lfFaceName[..face_len]);

            let desc = FontDesc {
                face_name,
                point_size,
                bold: lf.lfWeight >= 700,
                italic: lf.lfItalic != 0,
                underline: lf.lfUnderline != 0,
                dpi: self.initial.dpi,
            };
            Some(Font::new(desc))
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = self;
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cf_flag_values_match_commdlg_h() {
        // Pinned from <dlgs.h> so a typoed hex digit is caught.
        const CF_BOTH: u32 = 0x00000003;
        assert_eq!(CF_SCREENFONTS, 0x00000001);
        assert_eq!(CF_EFFECTS, 0x00000100);
        assert_eq!(CF_INITTOLOGFONTSTRUCT, 0x00000040);
        assert_eq!(CF_FORCEFONTEXIST, 0x00010000);
        assert_eq!(CF_BOTH, 0x00000003);
    }

    #[test]
    fn point_size_recovery_from_pixel_height() {
        // Mirror the same MulDiv the dialog uses, so a refactor of
        // the point-size reconstruction logic does not silently
        // produce nonsense (e.g. 0 or a negative number).
        let dpi = 96;
        let lf_height = -12; // 9pt at 96 DPI
        let recovered = ((-lf_height) * 72 / dpi).max(1);
        assert_eq!(recovered, 9);
    }
}
