//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Custom font (face name, point size, weight, style).
//!
//! A `Font` is a resource that can be passed to widgets to change the
//! text rendering style. On Windows it is a thin wrapper around `HFONT`;
//! the actual font is realised lazily when the first widget attaches
//! the font to its `WM_SETFONT` message.

#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{
    CreateFontW, DeleteObject, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET,
    DEFAULT_PITCH, FF_DONTCARE, FW_BOLD, FW_NORMAL, HFONT, LF_FACESIZE, LOGFONTW,
    OUT_DEFAULT_PRECIS,
};

/// Logical font attributes used to describe a `Font`.
///
/// Point size is expressed in points (1/72 of an inch). The Win32
/// logical unit (pixel height) is computed at construction time using
/// the supplied `dpi` (defaults to 96, the standard Windows DPI).
#[derive(Debug, Clone)]
pub struct FontDesc {
    /// Family name (e.g. `"Segoe UI"`, `"Arial"`).
    pub face_name: String,
    /// Point size (1 point = 1/72 inch).
    pub point_size: i32,
    /// `true` for bold, `false` for normal weight.
    pub bold: bool,
    /// `true` for italic, `false` for upright.
    pub italic: bool,
    /// `true` for underlined, `false` for plain.
    pub underline: bool,
    /// Dots-per-inch used to convert points → pixels (96 is standard).
    pub dpi: i32,
}

impl FontDesc {
    /// Build a `FontDesc` for the given face and point size (normal weight,
    /// no italic/underline, standard DPI of 96).
    pub fn new(face_name: &str, point_size: i32) -> Self {
        Self {
            face_name: face_name.to_string(),
            point_size,
            bold: false,
            italic: false,
            underline: false,
            dpi: 96,
        }
    }

    /// Mark the font as bold.
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// Mark the font as italic.
    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    /// Mark the font as underlined.
    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    /// Override the DPI used to convert points → pixels.
    pub fn with_dpi(mut self, dpi: i32) -> Self {
        self.dpi = dpi;
        self
    }
}

impl Default for FontDesc {
    fn default() -> Self {
        Self::new("Segoe UI", 9)
    }
}

/// A realised font. Holds an `HFONT` on Windows; owns the handle and
/// deletes it on `Drop`.
pub struct Font {
    #[cfg(target_os = "windows")]
    hfont: HFONT,
    desc: FontDesc,
}

impl Font {
    /// Build a new font from the given description.
    #[cfg(target_os = "windows")]
    pub fn new(desc: FontDesc) -> Self {
        let hfont = create_hfont(&desc);
        Font { hfont, desc }
    }

    /// Non-Windows stub.
    #[cfg(not(target_os = "windows"))]
    pub fn new(desc: FontDesc) -> Self {
        Font { desc }
    }

    /// Convenience: create a default `Segoe UI 9pt` font.
    pub fn default_system() -> Self {
        Self::new(FontDesc::default())
    }

    /// The underlying Win32 `HFONT` handle (or `0` on non-Windows).
    #[cfg(target_os = "windows")]
    pub fn hfont(&self) -> HFONT {
        self.hfont
    }

    /// The underlying Win32 `HFONT` handle (or `0` on non-Windows).
    #[cfg(not(target_os = "windows"))]
    pub fn hfont(&self) -> isize {
        0
    }

    /// Read-only access to the description.
    pub fn desc(&self) -> &FontDesc {
        &self.desc
    }
}

impl Clone for Font {
    fn clone(&self) -> Self {
        // Each clone gets its own HFONT (a HFONT can be selected into
        // multiple DCs, but DeleteObject must match the Create… count
        // to avoid double-free when more than one clone is dropped).
        #[cfg(target_os = "windows")]
        let hfont = create_hfont(&self.desc);
        #[cfg(not(target_os = "windows"))]
        let hfont = 0;
        Font {
            #[cfg(target_os = "windows")]
            hfont,
            desc: self.desc.clone(),
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for Font {
    fn drop(&mut self) {
        if !self.hfont.is_null() {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                DeleteObject(self.hfont);
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn create_hfont(desc: &FontDesc) -> HFONT {
    // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
    unsafe {
        // Convert points → logical units (pixels at 96 DPI by default):
        //   lfHeight = -MulDiv(point_size, dpi, 72)
        // The negative sign asks the font mapper for the *character*
        // height (excluding internal leading) rather than the line height.
        let pixel_height = -((desc.point_size * desc.dpi) / 72);

        let mut lf: LOGFONTW = std::mem::zeroed();
        lf.lfHeight = pixel_height;
        lf.lfWeight = if desc.bold {
            FW_BOLD as i32
        } else {
            FW_NORMAL as i32
        };
        lf.lfItalic = if desc.italic { 1 } else { 0 };
        lf.lfUnderline = if desc.underline { 1 } else { 0 };
        lf.lfCharSet = DEFAULT_CHARSET;
        lf.lfOutPrecision = OUT_DEFAULT_PRECIS;
        lf.lfClipPrecision = CLIP_DEFAULT_PRECIS;
        lf.lfQuality = CLEARTYPE_QUALITY;
        lf.lfPitchAndFamily = DEFAULT_PITCH | FF_DONTCARE;

        // Copy the face name (must be NUL-terminated and ≤ LF_FACESIZE - 1 chars).
        let face = desc.face_name.encode_utf16();
        for (i, unit) in face.enumerate() {
            if i >= LF_FACESIZE as usize - 1 {
                break;
            }
            lf.lfFaceName[i] = unit;
        }

        CreateFontW(
            lf.lfHeight,
            0, // lfWidth (0 = let the font mapper pick)
            0, // lfEscapement
            0, // lfOrientation
            lf.lfWeight,
            lf.lfItalic as u32,
            lf.lfUnderline as u32,
            0, // lfStrikeOut
            lf.lfCharSet as u32,
            lf.lfOutPrecision as u32,
            lf.lfClipPrecision as u32,
            lf.lfQuality as u32,
            lf.lfPitchAndFamily as u32,
            lf.lfFaceName.as_ptr(),
        )
    }
}
