//! Modal colour-chooser dialog (`wxColourDialog`).
//!
//! Wraps the Win32 common dialog `ChooseColorW` (comdlg32.dll). This
//! is the modal sibling of [`crate::colour_picker_ctrl`], which uses
//! the same underlying Win32 call but embeds it inside a button. The
//! dialog is *not* a wrapper around [`ColourPickerCtrl`] — it is a
//! stand-alone "show me the standard Windows colour picker" helper.
//!
//! See also: [`crate::colour_picker_ctrl`] for an in-frame
//! colour-picker button that opens the same dialog on click.

use crate::frame::Frame;
use crate::geometry::Colour;

#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Controls::Dialogs::{
    ChooseColorW, CC_ANYCOLOR, CC_FULLOPEN, CC_RGBINIT, CHOOSECOLORW,
};

/// A modal colour-picker dialog.
///
/// Build the dialog with setter methods, then call
/// [`ColorDialog::show_modal`] to present it. The selected colour is
/// returned as an `Option<Colour>` — `None` if the user cancelled.
pub struct ColorDialog {
    #[cfg(target_os = "windows")]
    parent_hwnd: HWND,
    title: String,
    initial: Colour,
    /// Custom-colour palette for `ChooseColorW`. `ChooseColorW` writes
    /// the last 16 user-picked colours into this buffer, so we keep
    /// it on the dialog struct (it must outlive the call).
    custom_colors: [u32; 16],
    /// Open with the full colour picker visible.
    full_open: bool,
    /// Allow the user to type any RGB value.
    any_color: bool,
}

impl ColorDialog {
    /// Create a new colour dialog associated with the given frame.
    pub fn new(frame: &Frame) -> Self {
        ColorDialog {
            #[cfg(target_os = "windows")]
            parent_hwnd: frame.hwnd(),
            title: String::new(),
            initial: Colour::BLACK,
            custom_colors: [0xFFFFFF; 16],
            full_open: true,
            any_color: true,
        }
    }

    /// Set the initial colour (the one the picker shows on open).
    pub fn set_initial_color(&mut self, colour: Colour) {
        self.initial = colour;
    }

    /// Set the dialog title. The Windows common dialog does not
    /// display a custom title — the title is stored so a future
    /// cross-platform wrapper can honour it; on Windows it is
    /// silently ignored.
    pub fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    /// If `true` (default), the full colour picker is shown
    /// (`CC_FULLOPEN`). If `false`, the user starts in the "basic"
    /// view and clicks "Define Custom Colours" to expand.
    pub fn set_full_open(&mut self, full_open: bool) {
        self.full_open = full_open;
    }

    /// If `true` (default), the user can type any RGB value
    /// (`CC_ANYCOLOR`). If `false`, the dialog is restricted to the
    /// basic set of 48 colours.
    pub fn set_any_color(&mut self, any_color: bool) {
        self.any_color = any_color;
    }

    /// Show the dialog modally. Returns the chosen colour, or
    /// `None` if the user cancelled.
    pub fn show_modal(&mut self) -> Option<Colour> {
        #[cfg(target_os = "windows")]
        {
            let mut flags: u32 = CC_RGBINIT;
            if self.full_open {
                flags |= CC_FULLOPEN;
            }
            if self.any_color {
                flags |= CC_ANYCOLOR;
            }

            // SAFETY: Win32 FFI call with validated arguments.
            unsafe {
                let initial = self.initial.to_colorref();
                self.custom_colors[0] = initial;
                let mut cc = CHOOSECOLORW {
                    lStructSize: std::mem::size_of::<CHOOSECOLORW>() as u32,
                    hwndOwner: self.parent_hwnd,
                    hInstance: std::ptr::null_mut(),
                    rgbResult: initial,
                    lpCustColors: self.custom_colors.as_mut_ptr(),
                    Flags: flags,
                    lCustData: 0,
                    lpTemplateName: std::ptr::null(),
                    lpfnHook: None,
                };
                let ok = ChooseColorW(&mut cc);
                if ok == 0 {
                    return None;
                }
                let r = (cc.rgbResult & 0xFF) as u8;
                let g = ((cc.rgbResult >> 8) & 0xFF) as u8;
                let b = ((cc.rgbResult >> 16) & 0xFF) as u8;
                Some(Colour::new(r, g, b, 255))
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = self;
            // Silence the unused warning on the `title` field.
            let _ = to_wide(&self.title);
            None
        }
    }
}

// Mark `to_wide` as used on non-Windows (we silence the unused
// import via the `_ = to_wide(...)` line in the cross-platform
// branch).
#[allow(dead_code)]
fn _unused_to_wide_marker(s: &str) -> Vec<u16> {
    crate::platform::win32::to_wide(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_state_default() {
        // We can't construct a `ColorDialog` without a real `Frame`,
        // so the unit tests are limited to the pure-Rust flag/path
        // logic. The actual `show_modal` requires a real Win32 dialog
        // and is covered by the windowed smoke test in
        // `examples/showcase_all.rs`.
        let mut flags: u32 = CC_RGBINIT;
        flags |= CC_FULLOPEN;
        assert_ne!(flags & CC_FULLOPEN, 0);
        flags &= !CC_FULLOPEN;
        assert_eq!(flags & CC_FULLOPEN, 0);
    }

    #[test]
    fn cc_flag_values_match_commdlg_h() {
        // Pinned from <dlgs.h> so a typoed hex digit is caught.
        assert_eq!(CC_RGBINIT, 0x00000001);
        assert_eq!(CC_FULLOPEN, 0x00000002);
        assert_eq!(CC_ANYCOLOR, 0x00000100);
    }
}
