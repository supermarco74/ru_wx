//! Modal dialog stubs for WinUI3.
//!
//! All dialogs return `Option<T>` (`Some(value)` on OK, `None` on cancel /
//! failure) matching the Win32 `DialogBoxParamW` and
//! `GetOpenFileNameW` / `ChooseColorW` semantics.
//!
//! The implementations reuse the standard Windows common dialogs
//! (`comdlg32.dll` for file-picker / color-picker, `shell32.dll` for
//! folder-picker, `MessageBoxW` from `user32.dll`).

#![allow(non_snake_case)]

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Controls::Dialogs::{ChooseColorW, CHOOSECOLORW};
use windows_sys::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, GetSaveFileNameW, OPENFILENAMEW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageBoxIcon {
    None,
    Information,
    Warning,
    Error,
    Question,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageBoxButton {
    Ok,
    OkCancel,
    YesNo,
    YesNoCancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageBoxResponse {
    Ok,
    Cancel,
    Yes,
    No,
}

const MB_OK: u32 = 0x00000000;
const MB_OKCANCEL: u32 = 0x00000001;
const MB_YESNO: u32 = 0x00000004;
const MB_YESNOCANCEL: u32 = 0x00000003;
const MB_ICONINFORMATION: u32 = 0x00000040;
const MB_ICONWARNING: u32 = 0x00000030;
const MB_ICONERROR: u32 = 0x00000010;
const MB_ICONQUESTION: u32 = 0x00000020;
const IDOK: i32 = 1;
const IDCANCEL: i32 = 2;
const IDYES: i32 = 6;
const IDNO: i32 = 7;

/// Show a modal message box anchored to `hwnd` (or desktop if null).
#[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper; `hwnd` may be null (desktop)
pub fn message_box(
    hwnd: HWND,
    title: &str,
    text: &str,
    buttons: MessageBoxButton,
    icon: MessageBoxIcon,
) -> MessageBoxResponse {
    let title_w = to_wide(title);
    let text_w = to_wide(text);
    let mut style = match buttons {
        MessageBoxButton::Ok => MB_OK,
        MessageBoxButton::OkCancel => MB_OKCANCEL,
        MessageBoxButton::YesNo => MB_YESNO,
        MessageBoxButton::YesNoCancel => MB_YESNOCANCEL,
    };
    style |= match icon {
        MessageBoxIcon::None => 0,
        MessageBoxIcon::Information => MB_ICONINFORMATION,
        MessageBoxIcon::Warning => MB_ICONWARNING,
        MessageBoxIcon::Error => MB_ICONERROR,
        MessageBoxIcon::Question => MB_ICONQUESTION,
    };
    let answer = unsafe { MessageBoxW(hwnd, text_w.as_ptr(), title_w.as_ptr(), style) };
    match answer {
        IDOK => MessageBoxResponse::Ok,
        IDCANCEL => MessageBoxResponse::Cancel,
        IDYES => MessageBoxResponse::Yes,
        IDNO => MessageBoxResponse::No,
        _ => MessageBoxResponse::Ok,
    }
}

// ============================================================================
// File picker (Open).
//
// Returns the chosen path on OK, `None` on cancel / failure.
// We use the modern `IFileOpenDialog` interface via `CoTaskMemFree` —
// the Win32 `OPENFILENAMEW` struct remains the public shape.
pub fn file_pick_open(hwnd: HWND, title: &str, filter: &str) -> Option<String> {
    // Buffers must outlive the GetOpenFileNameW call below; binding them
    // to local `let`s avoids the temporary-`Vec<u16>` dangling pointer
    // pattern flagged by clippy (and the use-after-free it would cause).
    let wide_title = to_wide(title);
    let wide_filter = to_wide(filter);
    unsafe {
        let mut name = [0u16; 1024];
        let mut ofn: OPENFILENAMEW = std::mem::zeroed();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = hwnd;
        ofn.lpstrFile = name.as_mut_ptr();
        ofn.nMaxFile = name.len() as u32;
        ofn.lpstrTitle = wide_title.as_ptr();
        ofn.lpstrFilter = wide_filter.as_ptr();
        ofn.Flags = 0x00001000 /* OFN_FILEMUSTEXIST */ | 0x00000004 /* OFN_EXPLORER */;
        if GetOpenFileNameW(&mut ofn) != 0 {
            let len = (0..name.len())
                .position(|i| name[i] == 0)
                .unwrap_or(name.len());
            Some(String::from_utf16_lossy(&name[..len]))
        } else {
            None
        }
    }
}

// ============================================================================
// File picker (Save).
pub fn file_pick_save(
    hwnd: HWND,
    title: &str,
    filter: &str,
    suggested: Option<&str>,
) -> Option<String> {
    // Buffers must outlive the GetSaveFileNameW call below; see
    // `file_pick_open` for the dangling-pointer rationale.
    let wide_title = to_wide(title);
    let wide_filter = to_wide(filter);
    unsafe {
        let mut name: [u16; 1024] = match suggested {
            Some(s) => s
                .encode_utf16()
                .chain(std::iter::once(0))
                .take(1024)
                .collect::<Vec<u16>>()
                .try_into()
                .unwrap_or([0u16; 1024]),
            None => [0u16; 1024],
        };
        let mut ofn: OPENFILENAMEW = std::mem::zeroed();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = hwnd;
        ofn.lpstrFile = name.as_mut_ptr();
        ofn.nMaxFile = name.len() as u32;
        ofn.lpstrTitle = wide_title.as_ptr();
        ofn.lpstrFilter = wide_filter.as_ptr();
        ofn.Flags = 0x00000002 /* OFN_OVERWRITEPROMPT */ | 0x00000004 /* OFN_EXPLORER */;
        if GetSaveFileNameW(&mut ofn) != 0 {
            let len = (0..name.len())
                .position(|i| name[i] == 0)
                .unwrap_or(name.len());
            Some(String::from_utf16_lossy(&name[..len]))
        } else {
            None
        }
    }
}

// ============================================================================
// Color picker.
//
// Returns the chosen RGB triple and the picked state (some users press
// Cancel and we should honour that).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color(pub u8, pub u8, pub u8);

pub fn pick_color(hwnd: HWND, initial: Color) -> Option<Color> {
    unsafe {
        let mut cc: CHOOSECOLORW = std::mem::zeroed();
        cc.lStructSize = std::mem::size_of::<CHOOSECOLORW>() as u32;
        cc.hwndOwner = hwnd;
        cc.rgbResult = (initial.0 as u32) | ((initial.1 as u32) << 8) | ((initial.2 as u32) << 16);
        cc.Flags = 0x00000001 /* CC_RGBINIT */ | 0x00000002 /* CC_FULLOPEN */;
        let mut custom_colors: [u32; 16] = [0; 16];
        cc.lpCustColors = custom_colors.as_mut_ptr();
        if ChooseColorW(&mut cc) != 0 {
            Some(Color(
                (cc.rgbResult & 0xFF) as u8,
                ((cc.rgbResult >> 8) & 0xFF) as u8,
                ((cc.rgbResult >> 16) & 0xFF) as u8,
            ))
        } else {
            None
        }
    }
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ============================================================================
// Re-export for the high-level user code: a single `dialogs` module.
pub use self::{
    file_pick_open as open_file, file_pick_save as save_file, message_box as msgbox,
    pick_color as color_pick,
};
pub use self::{
    MessageBoxButton as Buttons, MessageBoxIcon as Icon, MessageBoxResponse as Response,
};
