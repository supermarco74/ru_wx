//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! MessageBox wrapper around the Win32 `MessageBoxW` API / macOS `NSAlert`.

use crate::window::frame::Frame;

#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[cfg(target_os = "macos")]
use objc2_app_kit::{NSAlert, NSAlertFirstButtonReturn, NSAlertSecondButtonReturn, NSAlertStyle};
#[cfg(target_os = "macos")]
use objc2_foundation::{MainThreadMarker, NSString};

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
                crate::platform::window_handle_as_hwnd(parent),
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

    #[cfg(target_os = "macos")]
    {
        let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
        let alert = NSAlert::new(mtm);
        alert.setMessageText(&NSString::from_str(title));
        alert.setInformativeText(&NSString::from_str(message));
        alert.setAlertStyle(match icon {
            MessageBoxIcon::Information => NSAlertStyle::Informational,
            MessageBoxIcon::Warning => NSAlertStyle::Warning,
            MessageBoxIcon::Error => NSAlertStyle::Critical,
            MessageBoxIcon::Question => NSAlertStyle::Informational,
        });

        match style {
            MessageBoxStyle::Ok => {
                alert.addButtonWithTitle(&NSString::from_str("OK"));
            }
            MessageBoxStyle::OkCancel => {
                alert.addButtonWithTitle(&NSString::from_str("OK"));
                alert.addButtonWithTitle(&NSString::from_str("Cancel"));
            }
            MessageBoxStyle::YesNo => {
                alert.addButtonWithTitle(&NSString::from_str("Yes"));
                alert.addButtonWithTitle(&NSString::from_str("No"));
            }
            MessageBoxStyle::YesNoCancel => {
                alert.addButtonWithTitle(&NSString::from_str("Yes"));
                alert.addButtonWithTitle(&NSString::from_str("No"));
                alert.addButtonWithTitle(&NSString::from_str("Cancel"));
            }
        }

        let _ = parent;
        let response = alert.runModal();
        match style {
            MessageBoxStyle::Ok => MessageBoxResult::Ok,
            MessageBoxStyle::OkCancel => {
                if response == NSAlertFirstButtonReturn {
                    MessageBoxResult::Ok
                } else {
                    MessageBoxResult::Cancel
                }
            }
            MessageBoxStyle::YesNo => {
                if response == NSAlertFirstButtonReturn {
                    MessageBoxResult::Yes
                } else {
                    MessageBoxResult::No
                }
            }
            MessageBoxStyle::YesNoCancel => {
                if response == NSAlertFirstButtonReturn {
                    MessageBoxResult::Yes
                } else if response == NSAlertSecondButtonReturn {
                    MessageBoxResult::No
                } else {
                    MessageBoxResult::Cancel
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        crate::platform::gtk::show_message_box(parent.hwnd(), message, title, style, icon)
    }

    #[cfg(all(
        not(target_os = "windows"),
        not(target_os = "macos"),
        not(target_os = "linux")
    ))]
    {
        let _ = (parent, message, title, style, icon);
        MessageBoxResult::Cancel
    }
}
