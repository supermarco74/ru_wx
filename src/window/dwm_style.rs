//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Windows 11 modern-appearance helpers, available on every
//! [`Frame`] (and therefore on every window type that wraps one —
//! `TopLevelWindow`, dialogs, MDI parents, …).
//!
//! Three DWM attributes drive the "modern" Win11 look:
//!
//! * **Dark title bar** — `DWMWA_USE_IMMERSIVE_DARK_MODE` (attr 20):
//!   paints the non-client caption with the dark theme. Available
//!   since Windows 10 1809 (build 17763) / Windows 11.
//! * **Backdrop material** — `DWMWA_SYSTEMBACKDROP_TYPE` (attr 38):
//!   Mica / Acrylic / Mica-Alt behind the client area. Windows 11
//!   22H2 (build 22621) and later; earlier releases return an error
//!   and the window keeps its opaque background.
//! * **Rounded corners** — `DWMWA_WINDOW_CORNER_PREFERENCE` (attr
//!   33): see [`WindowCornerPreference`]. Windows 11 only (accepted
//!   but ignored on Windows 10 1809+).
//!
//! All setters return `bool` (`true` = the DWM accepted the call) so
//! callers can detect older Windows releases and fall back
//! gracefully. Everything is a no-op returning `false` on
//! non-Windows targets.
//!
//! # Example
//! ```no_run
//! use ru_wx::prelude::*;
//! use ru_wx::{Appearance, BackdropType, Frame, WindowCornerPreference};
//!
//! let frame = Frame::builder().with_title("Modern").build();
//! // One call: dark title bar following the OS setting + Mica.
//! frame.apply_modern_style();
//! // …or piece by piece:
//! frame.set_dark_title_bar(Appearance::System.resolve());
//! frame.set_backdrop(BackdropType::Mica);
//! frame.set_corner_preference(WindowCornerPreference::Round);
//! ```

use crate::window::frame::Frame;
use crate::window::top_level_window::WindowCornerPreference;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Dwm::{
    DwmGetWindowAttribute, DwmSetWindowAttribute, DWMSBT_AUTO, DWMSBT_MAINWINDOW, DWMSBT_NONE,
    DWMSBT_TABBEDWINDOW, DWMSBT_TRANSIENTWINDOW, DWMWA_SYSTEMBACKDROP_TYPE,
    DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE, DWM_SYSTEMBACKDROP_TYPE,
    DWM_WINDOW_CORNER_PREFERENCE,
};

/// The Windows 11 backdrop material drawn behind the client area
/// (`DWMWA_SYSTEMBACKDROP_TYPE`). Values mirror the Win32
/// `DWM_SYSTEMBACKDROP_TYPE` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackdropType {
    /// `DWMSBT_AUTO` — let the DWM decide (usually Mica for
    /// top-level app windows).
    #[default]
    Auto,
    /// `DWMSBT_NONE` — opaque, classic background.
    None,
    /// `DWMSBT_MAINWINDOW` — Mica (the Win11 main-window material).
    Mica,
    /// `DWMSBT_TRANSIENTWINDOW` — Acrylic (transient surfaces:
    /// flyouts, context menus).
    Acrylic,
    /// `DWMSBT_TABBEDWINDOW` — Mica Alt (tabbed title bar variant).
    MicaAlt,
}

#[cfg(target_os = "windows")]
impl BackdropType {
    pub(crate) fn to_win32(self) -> DWM_SYSTEMBACKDROP_TYPE {
        match self {
            BackdropType::Auto => DWMSBT_AUTO,
            BackdropType::None => DWMSBT_NONE,
            BackdropType::Mica => DWMSBT_MAINWINDOW,
            BackdropType::Acrylic => DWMSBT_TRANSIENTWINDOW,
            BackdropType::MicaAlt => DWMSBT_TABBEDWINDOW,
        }
    }
}

// ── Raw-HWND helpers shared by every window wrapper ──────────────────
//
// All the DWM attribute plumbing lives here once; the `Frame`,
// `Dialog`, `MDIParentFrame` and `MiniFrame` methods below are thin
// forwarding wrappers around these.

#[cfg(target_os = "windows")]
pub(crate) fn set_dark_title_bar_hwnd(hwnd: windows_sys::Win32::Foundation::HWND, dark: bool) -> bool {
    let value: i32 = if dark { 1 } else { 0 };
    // SAFETY: caller passes a live HWND; the DWM reads the BOOL-sized
    // value once before returning, and `cbattribute` matches
    // `size_of::<i32>` as the contract requires.
    let hr = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
            &value as *const _ as *const _,
            std::mem::size_of::<i32>() as u32,
        )
    };
    hr >= 0
}

#[cfg(target_os = "windows")]
pub(crate) fn dark_title_bar_hwnd(hwnd: windows_sys::Win32::Foundation::HWND) -> Option<bool> {
    let mut value: i32 = 0;
    // SAFETY: live HWND; the DWM writes a BOOL-sized value into the
    // out pointer, whose size matches `cbattribute`.
    let hr = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
            &mut value as *mut _ as *mut _,
            std::mem::size_of::<i32>() as u32,
        )
    };
    (hr >= 0).then_some(value != 0)
}

#[cfg(target_os = "windows")]
pub(crate) fn set_backdrop_hwnd(
    hwnd: windows_sys::Win32::Foundation::HWND,
    backdrop: BackdropType,
) -> bool {
    let value: DWM_SYSTEMBACKDROP_TYPE = backdrop.to_win32();
    // SAFETY: live HWND; the DWM reads the i32-sized value once, and
    // `cbattribute` matches its size.
    let hr = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE as u32,
            &value as *const _ as *const _,
            std::mem::size_of::<DWM_SYSTEMBACKDROP_TYPE>() as u32,
        )
    };
    hr >= 0
}

#[cfg(target_os = "windows")]
pub(crate) fn set_corner_preference_hwnd(
    hwnd: windows_sys::Win32::Foundation::HWND,
    pref: WindowCornerPreference,
) -> bool {
    let value: DWM_WINDOW_CORNER_PREFERENCE = pref.to_win32();
    // SAFETY: live HWND; value read once by the DWM; size matches.
    let hr = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            &value as *const _ as *const _,
            std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        )
    };
    hr >= 0
}

#[cfg(target_os = "windows")]
pub(crate) fn corner_preference_hwnd(
    hwnd: windows_sys::Win32::Foundation::HWND,
) -> Option<WindowCornerPreference> {
    let mut value: DWM_WINDOW_CORNER_PREFERENCE = 0;
    // SAFETY: live HWND; out pointer sized exactly as declared.
    let hr = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            &mut value as *mut _ as *mut _,
            std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        )
    };
    if hr < 0 {
        return None;
    }
    WindowCornerPreference::from_win32(value)
}

/// Apply dark-titlebar (per OS theme) + Mica + default corners to a
/// raw HWND. Returns `true` if the dark-title-bar attribute was
/// accepted.
#[cfg(target_os = "windows")]
pub(crate) fn apply_modern_style_hwnd(hwnd: windows_sys::Win32::Foundation::HWND) -> bool {
    let dark = crate::core::appearance::Appearance::System.resolve();
    let ok = set_dark_title_bar_hwnd(hwnd, dark);
    set_backdrop_hwnd(hwnd, BackdropType::Mica);
    set_corner_preference_hwnd(hwnd, WindowCornerPreference::Default);
    ok
}

/// Implement the five modern-style methods on a window wrapper that
/// exposes `fn hwnd(&self) -> HWND`.
macro_rules! impl_modern_style {
    ($ty:ty) => {
        impl_modern_style!($ty, hwnd);
    };
    ($ty:ty, $hwnd:ident) => {
        impl $ty {
            /// Paint the title bar with the dark theme
            /// (`DWMWA_USE_IMMERSIVE_DARK_MODE`). Returns `true` if
            /// the DWM accepted the attribute (Windows 10 1809+).
            #[cfg(target_os = "windows")]
            pub fn set_dark_title_bar(&self, dark: bool) -> bool {
                set_dark_title_bar_hwnd(self.$hwnd(), dark)
            }
            #[cfg(not(target_os = "windows"))]
            pub fn set_dark_title_bar(&self, _dark: bool) -> bool {
                false
            }

            /// Read back the current dark-title-bar flag. `None`
            /// when the DWM does not support the attribute.
            #[cfg(target_os = "windows")]
            pub fn dark_title_bar(&self) -> Option<bool> {
                dark_title_bar_hwnd(self.$hwnd())
            }
            #[cfg(not(target_os = "windows"))]
            pub fn dark_title_bar(&self) -> Option<bool> {
                None
            }

            /// Set the Windows 11 backdrop material
            /// (`DWMWA_SYSTEMBACKDROP_TYPE`). Returns `true` if the
            /// DWM accepted the attribute (Windows 11 22H2+).
            #[cfg(target_os = "windows")]
            pub fn set_backdrop(&self, backdrop: BackdropType) -> bool {
                set_backdrop_hwnd(self.$hwnd(), backdrop)
            }
            #[cfg(not(target_os = "windows"))]
            pub fn set_backdrop(&self, _backdrop: BackdropType) -> bool {
                false
            }

            /// Set the Windows 11 rounded-corner preference
            /// (`DWMWA_WINDOW_CORNER_PREFERENCE`). Returns `true` if
            /// the DWM accepted the call.
            #[cfg(target_os = "windows")]
            pub fn set_corner_preference(&self, pref: WindowCornerPreference) -> bool {
                set_corner_preference_hwnd(self.$hwnd(), pref)
            }
            #[cfg(not(target_os = "windows"))]
            pub fn set_corner_preference(&self, _pref: WindowCornerPreference) -> bool {
                false
            }

            /// Read back the current corner preference. `None` when
            /// the DWM does not support the attribute.
            #[cfg(target_os = "windows")]
            pub fn corner_preference(&self) -> Option<WindowCornerPreference> {
                corner_preference_hwnd(self.$hwnd())
            }
            #[cfg(not(target_os = "windows"))]
            pub fn corner_preference(&self) -> Option<WindowCornerPreference> {
                None
            }

            /// Apply the full Windows 11 modern style in one call:
            /// dark title bar following the OS appearance, Mica
            /// backdrop and default (rounded) corners. Each step
            /// degrades gracefully on older Windows releases.
            #[cfg(target_os = "windows")]
            pub fn apply_modern_style(&self) -> bool {
                apply_modern_style_hwnd(self.$hwnd())
            }
            #[cfg(not(target_os = "windows"))]
            pub fn apply_modern_style(&self) -> bool {
                false
            }
        }
    };
}

impl_modern_style!(Frame);
impl_modern_style!(crate::window::dialog::Dialog);
impl_modern_style!(crate::window::mdi::MDIParentFrame, parent_hwnd);
impl_modern_style!(crate::window::frame_extras::MiniFrame);


#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    /// Pin the DWM attribute ids and backdrop values to the
    /// `dwmapi.h` definitions so a windows-sys upgrade (or a manual
    /// edit) cannot silently change what we send to the DWM.
    #[test]
    fn dwm_constants_match_dwmapi_h() {
        assert_eq!(DWMWA_USE_IMMERSIVE_DARK_MODE, 20);
        assert_eq!(DWMWA_WINDOW_CORNER_PREFERENCE, 33);
        assert_eq!(DWMWA_SYSTEMBACKDROP_TYPE, 38);
        assert_eq!(BackdropType::Auto.to_win32(), 0);
        assert_eq!(BackdropType::None.to_win32(), 1);
        assert_eq!(BackdropType::Mica.to_win32(), 2);
        assert_eq!(BackdropType::Acrylic.to_win32(), 3);
        assert_eq!(BackdropType::MicaAlt.to_win32(), 4);
    }
}
