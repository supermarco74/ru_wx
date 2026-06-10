//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! MessageBox wrapper around the Win32 `MessageBoxW` API.

use crate::window::frame::Frame;

#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// MessageBox style flags
#[cfg(target_os = "windows")]
const MB_OK: u32 = 0;
#[cfg(target_os = "windows")]
const MB_OKCANCEL: u32 = 1;
#[cfg(target_os = "windows")]
const MB_YESNOCANCEL: u32 = 3;
#[cfg(target_os = "windows")]
const MB_YESNO: u32 = 4;
#[cfg(target_os = "windows")]
const MB_ICONINFORMATION: u32 = 0x40;
#[cfg(target_os = "windows")]
const MB_ICONWARNING: u32 = 0x30;
#[cfg(target_os = "windows")]
const MB_ICONERROR: u32 = 0x10;
#[cfg(target_os = "windows")]
const MB_ICONQUESTION: u32 = 0x20;

// MessageBox return values
#[cfg(target_os = "windows")]
const IDOK: i32 = 1;
#[cfg(target_os = "windows")]
const IDCANCEL: i32 = 2;
#[cfg(target_os = "windows")]
const IDYES: i32 = 6;
#[cfg(target_os = "windows")]
const IDNO: i32 = 7;

/// Button layout style for the message box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageBoxStyle {
    Ok,
    OkCancel,
    YesNo,
    YesNoCancel,
}

/// Icon displayed in the message box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageBoxIcon {
    Information,
    Warning,
    Error,
    Question,
}

/// Result of the message box interaction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageBoxResult {
    Ok,
    Cancel,
    Yes,
    No,
}

/// Display a modal message box as a child of the given frame.
///
/// The message box blocks until the user dismisses it, then returns
/// which button was clicked.
pub fn message_box(
    parent: &Frame,
    message: &str,
    title: &str,
    style: MessageBoxStyle,
    icon: MessageBoxIcon,
) -> MessageBoxResult {
    #[cfg(target_os = "windows")]
    {
        let wide_message = to_wide(message);
        let wide_title = to_wide(title);

        let style_flag = match style {
            MessageBoxStyle::Ok => MB_OK,
            MessageBoxStyle::OkCancel => MB_OKCANCEL,
            MessageBoxStyle::YesNo => MB_YESNO,
            MessageBoxStyle::YesNoCancel => MB_YESNOCANCEL,
        };

        let icon_flag = match icon {
            MessageBoxIcon::Information => MB_ICONINFORMATION,
            MessageBoxIcon::Warning => MB_ICONWARNING,
            MessageBoxIcon::Error => MB_ICONERROR,
            MessageBoxIcon::Question => MB_ICONQUESTION,
        };

        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let result = unsafe {
            MessageBoxW(
                parent.hwnd(),
                wide_message.as_ptr(),
                wide_title.as_ptr(),
                style_flag | icon_flag,
            )
        };

        match result {
            IDOK => MessageBoxResult::Ok,
            IDCANCEL => MessageBoxResult::Cancel,
            IDYES => MessageBoxResult::Yes,
            IDNO => MessageBoxResult::No,
            _ => MessageBoxResult::Cancel,
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (parent, message, title, style, icon);
        MessageBoxResult::Cancel
    }
}
