//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! System colour and metric queries (`wxSystemSettings`).

use crate::core::geometry::Colour;

/// System-wide settings (colours, metrics).
pub struct SystemSettings;

impl SystemSettings {
    /// Standard window background colour.
    #[cfg(target_os = "windows")]
    pub fn colour_window() -> Colour {
        sys_colour(5) // COLOR_WINDOW
    }

    #[cfg(not(target_os = "windows"))]
    pub fn colour_window() -> Colour {
        Colour::WHITE
    }

    /// Standard window text colour.
    #[cfg(target_os = "windows")]
    pub fn colour_window_text() -> Colour {
        sys_colour(8) // COLOR_WINDOWTEXT
    }

    #[cfg(not(target_os = "windows"))]
    pub fn colour_window_text() -> Colour {
        Colour::BLACK
    }

    /// Hyperlink blue (hotlight).
    #[cfg(target_os = "windows")]
    pub fn colour_hotlight() -> Colour {
        sys_colour(26) // COLOR_HOTLIGHT
    }

    #[cfg(not(target_os = "windows"))]
    pub fn colour_hotlight() -> Colour {
        Colour::new(0, 102, 204, 255)
    }

    /// Double-click rectangle width in pixels.
    #[cfg(target_os = "windows")]
    pub fn metric_double_click_width() -> i32 {
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics(36) }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn metric_double_click_width() -> i32 {
        4
    }
}

#[cfg(target_os = "windows")]
fn sys_colour(index: i32) -> Colour {
    let cref = unsafe { windows_sys::Win32::Graphics::Gdi::GetSysColor(index) };
    Colour::new(
        (cref & 0xFF) as u8,
        ((cref >> 8) & 0xFF) as u8,
        ((cref >> 16) & 0xFF) as u8,
        255,
    )
}
