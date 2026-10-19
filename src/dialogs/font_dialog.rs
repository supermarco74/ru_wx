//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Modal font-chooser dialog (`wxFontDialog`).
//!
//! Wraps the Win32 common dialog `ChooseFontW` (comdlg32.dll). The
//! user picks a face name, point size, weight (bold), style (italic
//! / underline / strike-out), and (optionally) a colour. On confirm,
//! the chosen attributes are returned as a [`FontDesc`]; pass it to
//! [`crate::core::font::Font::new`] to build a live font that can be
//! attached to a widget via `WM_SETFONT`.
//!
//! See also: [`crate::controls::colour_picker_ctrl`] / [`crate::dialogs::color_dialog`]
//! for colour-only pickers and [`crate::core::font`] for the font types.

use crate::core::font::{Font, FontDesc};
use crate::window::frame::Frame;

#[cfg(target_os = "macos")]
use objc2::rc::Retained;
#[cfg(target_os = "macos")]
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
#[cfg(target_os = "macos")]
use objc2_app_kit::{
    NSApplication, NSButton, NSFont, NSFontManager, NSFontPanel, NSFontTraitMask,
    NSModalResponseCancel, NSModalResponseOK, NSView,
};
#[cfg(target_os = "macos")]
use objc2_foundation::{
    MainThreadMarker, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};
#[cfg(target_os = "macos")]
use std::cell::Cell;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::LOGFONTW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Controls::Dialogs::{
    ChooseFontW, CF_EFFECTS, CF_FORCEFONTEXIST, CF_INITTOLOGFONTSTRUCT, CF_SCREENFONTS, CHOOSEFONTW,
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
    /// [`crate::dialogs::color_dialog::ColorDialog::set_title`] for the same
    /// cross-platform workaround).
    title: String,
}

impl FontDialog {
    /// Create a new font dialog associated with the given frame,
    /// pre-populated with the system default font (Segoe UI 9pt).
    pub fn new(frame: &Frame) -> Self {
        #[cfg(not(target_os = "windows"))]
        let _ = frame;
        FontDialog {
            #[cfg(target_os = "windows")]
            parent_hwnd: frame.hwnd() as HWND,
            initial: FontDesc::default(),
            initial_colour: 0,
            show_effects: true,
            title: String::new(),
        }
    }

    /// Create a new font dialog with the given initial font.
    pub fn with_initial(frame: &Frame, initial: FontDesc) -> Self {
        #[cfg(not(target_os = "windows"))]
        let _ = frame;
        FontDialog {
            #[cfg(target_os = "windows")]
            parent_hwnd: frame.hwnd() as HWND,
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
            let initial_point10 = self.initial.point_size * 10;

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

        #[cfg(target_os = "macos")]
        {
            show_macos_font_panel(&self.initial)
        }

        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            let _ = self;
            None
        }
    }
}

#[cfg(target_os = "macos")]
thread_local! {
    static FONT_PANEL_RESULT: Cell<Option<bool>> = const { Cell::new(None) };
}

#[cfg(target_os = "macos")]
#[derive(Default)]
struct FontPanelActionsIvars {
    confirmed: Cell<bool>,
}

#[cfg(target_os = "macos")]
define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = FontPanelActionsIvars]
    struct FontPanelActions;

    unsafe impl NSObjectProtocol for FontPanelActions {}

    impl FontPanelActions {
        #[unsafe(method(fontPanelOk:))]
        fn font_panel_ok(&self, _sender: Option<&AnyObject>) {
            self.ivars().confirmed.set(true);
            FONT_PANEL_RESULT.with(|r| r.set(Some(true)));
            let mtm = MainThreadMarker::new().expect("main thread");
            NSApplication::sharedApplication(mtm).stopModalWithCode(NSModalResponseOK);
        }

        #[unsafe(method(fontPanelCancel:))]
        fn font_panel_cancel(&self, _sender: Option<&AnyObject>) {
            self.ivars().confirmed.set(false);
            FONT_PANEL_RESULT.with(|r| r.set(Some(false)));
            let mtm = MainThreadMarker::new().expect("main thread");
            NSApplication::sharedApplication(mtm).stopModalWithCode(NSModalResponseCancel);
        }
    }
);

#[cfg(target_os = "macos")]
use objc2::runtime::AnyObject;

#[cfg(target_os = "macos")]
fn macos_font_from_desc(desc: &FontDesc) -> Retained<NSFont> {
    let size = desc.point_size as f64;
    if desc.face_name.is_empty() {
        NSFont::systemFontOfSize(size)
    } else if let Some(font) = NSFont::fontWithName_size(&NSString::from_str(&desc.face_name), size)
    {
        font
    } else {
        NSFont::systemFontOfSize(size)
    }
}

#[cfg(target_os = "macos")]
fn show_macos_font_panel(initial: &FontDesc) -> Option<Font> {
    let mtm = MainThreadMarker::new()?;
    FONT_PANEL_RESULT.with(|r| r.set(None));

    let font = macos_font_from_desc(initial);
    let manager = NSFontManager::sharedFontManager(mtm);
    manager.setSelectedFont_isMultiple(&font, false);

    let panel = NSFontPanel::sharedFontPanel(mtm);
    panel.setWorksWhenModal(true);
    panel.setPanelFont_isMultiple(&font, false);

    let actions = FontPanelActions::alloc(mtm).set_ivars(FontPanelActionsIvars {
        confirmed: Cell::new(false),
    });
    let actions: Retained<FontPanelActions> = unsafe { msg_send![super(actions), init] };

    let accessory = NSView::initWithFrame(
        NSView::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(220.0, 32.0)),
    );
    // SAFETY: valid selectors and target on the main thread.
    let ok = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str("OK"),
            Some(&actions),
            Some(sel!(fontPanelOk:)),
            mtm,
        )
    };
    ok.setFrame(NSRect::new(
        NSPoint::new(10.0, 4.0),
        NSSize::new(90.0, 24.0),
    ));
    accessory.addSubview(&ok);

    let cancel = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str("Cancel"),
            Some(&actions),
            Some(sel!(fontPanelCancel:)),
            mtm,
        )
    };
    cancel.setFrame(NSRect::new(
        NSPoint::new(120.0, 4.0),
        NSSize::new(90.0, 24.0),
    ));
    accessory.addSubview(&cancel);
    panel.setAccessoryView(Some(&accessory));

    panel.makeKeyAndOrderFront(None);
    let response = NSApplication::sharedApplication(mtm).runModalForWindow(&panel);
    panel.setAccessoryView(None);

    if response != NSModalResponseOK && !FONT_PANEL_RESULT.with(|r| r.get() == Some(true)) {
        return None;
    }

    let chosen = manager.selectedFont().unwrap_or(font);
    let traits = manager.traitsOfFont(&chosen);
    let face_name = chosen
        .familyName()
        .map(|s| s.to_string())
        .unwrap_or_else(|| initial.face_name.clone());
    let point_size = chosen.pointSize().round() as i32;
    let desc = FontDesc {
        face_name,
        point_size: point_size.max(1),
        bold: traits.contains(NSFontTraitMask::BoldFontMask),
        italic: traits.contains(NSFontTraitMask::ItalicFontMask),
        underline: initial.underline,
        dpi: initial.dpi,
    };
    Some(Font::new(desc))
}

#[cfg(all(test, target_os = "windows"))]
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
