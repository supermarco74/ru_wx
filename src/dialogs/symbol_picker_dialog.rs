//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Symbol-picker dialog (`wxSymbolPickerDialog`).
//!
//! A modal dialog that lets the user pick a single character / symbol
//! from a list. It is the ru_wx analogue of the
//! `wxSymbolPickerDialog` widget family (typically used for "Insert
//! Special Character" in editors).
//!
//! The dialog contains:
//!
//! * a prompt label,
//! * a listbox showing the supplied symbol list (each row is one
//!   symbol; rows can be a single Unicode codepoint, a short string,
//!   or a multi-character glyph if you wish),
//! * a single-line `EDIT` showing the currently selected symbol so
//!   the user can also type a character directly,
//! * "OK" and "Cancel" buttons.
//!
//! On OK the dialog returns the selected symbol as a `String`; on
//! cancel it returns `None`.
//!
//! # Example
//! ```no_run
//! use ru_wx::prelude::*;
//!
//! let frame = Frame::builder().with_title("App").with_size(100, 100).build();
//! // Common typographic symbols as a starter set.
//! let symbols = ["\u{2020}", "\u{2021}", "\u{2022}", "\u{2030}", "\u{20AC}"];
//! let dlg = SymbolPickerDialog::new(&frame, "Pick a symbol:", "Symbols", &symbols, 0);
//! if let Some(s) = dlg.show_modal() {
//!     println!("Picked: {s}");
//! }
//! ```

use std::cell::RefCell;
use std::rc::Rc;

use crate::window::frame::Frame;

use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{GetStockObject, UpdateWindow, DEFAULT_GUI_FONT};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::SystemServices::SS_LEFT;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{SetFocus, VK_ESCAPE, VK_RETURN};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── Win32 ListBox / Edit constants ────────────────────────────────────

#[cfg(target_os = "windows")]
const LB_ADDSTRING: u32 = 0x0180;
#[cfg(target_os = "windows")]
const LB_SETCURSEL: u32 = 0x0186;
#[cfg(target_os = "windows")]
const LB_GETCURSEL: u32 = 0x0188;
#[cfg(target_os = "windows")]
const LBS_NOTIFY: u32 = 0x0001;

// ── Shared dialog class constants ────────────────────────────────────

const IDOK_I: i32 = 1;
const IDCANCEL_I: i32 = 2;

/// Class name registered for the symbol-picker dialog.
#[cfg(target_os = "windows")]
const SYMBOL_CLASS_NAME: &str = "RuWxSymbolPickerDialogClass";

/// Dialog width.
const DLG_W: i32 = 360;
/// Dialog height.
const DLG_H: i32 = 320;
/// Padding.
const PAD: i32 = 10;
/// Label height.
const LABEL_H: i32 = 24;
/// Listbox height.
const LIST_H: i32 = 180;
/// Edit height.
const EDIT_H: i32 = 24;
/// Button width.
const BUTTON_W: i32 = 90;
/// Button height.
const BUTTON_H: i32 = 28;

// ── Inner type ────────────────────────────────────────────────────────

struct SymbolDialogInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(target_os = "windows")]
    hwnd_label: HWND,
    #[cfg(target_os = "windows")]
    hwnd_list: HWND,
    #[cfg(target_os = "windows")]
    hwnd_edit: HWND,
    /// `Some(symbol)` on OK, `None` on cancel.
    result: Option<String>,
    /// Set by WndProc when the dialog has finished.
    is_done: bool,
}

// ── Window class registration ────────────────────────────────────────

#[cfg(target_os = "windows")]
fn register_symbol_class() {
    // SAFETY: Win32 FFI call with validated arguments.
    unsafe {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_name = to_wide(SYMBOL_CLASS_NAME);

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(symbol_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: GetStockObject(5) as _, // NULL_BRUSH
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wc);
    }
}

// ── Builder ──────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn build_symbol_dialog(
    frame: &Frame,
    message: &str,
    caption: &str,
    symbols: &[&str],
    initial: Option<usize>,
) -> Rc<RefCell<SymbolDialogInner>> {
    register_symbol_class();

    // SAFETY: Win32 FFI calls with validated arguments.
    unsafe {
        let wide_class = to_wide(SYMBOL_CLASS_NAME);
        let wide_caption = to_wide(caption);
        let wide_msg = to_wide(message);
        let hinstance = GetModuleHandleW(std::ptr::null());
        let parent = frame.hwnd();

        let hwnd = CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            wide_class.as_ptr(),
            wide_caption.as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            DLG_W,
            DLG_H,
            parent,
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null_mut(),
        );

        // Label.
        let hwnd_label = CreateWindowExW(
            0,
            to_wide("STATIC").as_ptr(),
            wide_msg.as_ptr(),
            WS_CHILD | WS_VISIBLE | SS_LEFT,
            PAD,
            PAD,
            DLG_W - 2 * PAD,
            LABEL_H,
            hwnd,
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null_mut(),
        );

        // Listbox.
        let list_y = PAD + LABEL_H + 4;
        let hwnd_list = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            to_wide("LISTBOX").as_ptr(),
            std::ptr::null(),
            WS_CHILD | WS_VISIBLE | WS_BORDER | WS_VSCROLL | LBS_NOTIFY,
            PAD,
            list_y,
            DLG_W - 2 * PAD,
            LIST_H,
            hwnd,
            next_control_id() as usize as HMENU,
            hinstance,
            std::ptr::null_mut(),
        );
        for (i, s) in symbols.iter().enumerate() {
            let wide_s = to_wide(s);
            SendMessageW(hwnd_list, LB_ADDSTRING, 0, wide_s.as_ptr() as isize);
            if let Some(idx) = initial {
                if idx == i {
                    SendMessageW(hwnd_list, LB_SETCURSEL, i, 0);
                }
            }
        }

        // Edit.
        let edit_y = list_y + LIST_H + 10;
        let hwnd_edit = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            to_wide("EDIT").as_ptr(),
            std::ptr::null(),
            WS_CHILD | WS_VISIBLE | WS_BORDER | (ES_AUTOHSCROLL as u32) | (ES_CENTER as u32),
            PAD,
            edit_y,
            DLG_W - 2 * PAD,
            EDIT_H,
            hwnd,
            next_control_id() as usize as HMENU,
            hinstance,
            std::ptr::null_mut(),
        );

        // Buttons.
        let button_y = edit_y + EDIT_H + 14;
        let button_x_ok = DLG_W - PAD - 2 * BUTTON_W - 6;
        let button_x_cancel = DLG_W - PAD - BUTTON_W;
        let hfont = GetStockObject(DEFAULT_GUI_FONT);
        let wide_ok = to_wide("OK");
        let wide_cancel = to_wide("Cancel");
        let hwnd_ok = CreateWindowExW(
            0,
            to_wide("BUTTON").as_ptr(),
            wide_ok.as_ptr(),
            WS_CHILD | WS_VISIBLE | (BS_DEFPUSHBUTTON as u32),
            button_x_ok,
            button_y,
            BUTTON_W,
            BUTTON_H,
            hwnd,
            IDOK_I as usize as HMENU,
            hinstance,
            std::ptr::null_mut(),
        );
        let hwnd_cancel = CreateWindowExW(
            0,
            to_wide("BUTTON").as_ptr(),
            wide_cancel.as_ptr(),
            WS_CHILD | WS_VISIBLE | (BS_PUSHBUTTON as u32),
            button_x_cancel,
            button_y,
            BUTTON_W,
            BUTTON_H,
            hwnd,
            IDCANCEL_I as usize as HMENU,
            hinstance,
            std::ptr::null_mut(),
        );
        SendMessageW(hwnd_label, WM_SETFONT, hfont as usize, 1);
        SendMessageW(hwnd_list, WM_SETFONT, hfont as usize, 1);
        SendMessageW(hwnd_edit, WM_SETFONT, hfont as usize, 1);
        SendMessageW(hwnd_ok, WM_SETFONT, hfont as usize, 1);
        SendMessageW(hwnd_cancel, WM_SETFONT, hfont as usize, 1);
        SetFocus(hwnd_list);

        let inner = Rc::new(RefCell::new(SymbolDialogInner {
            hwnd,
            hwnd_label,
            hwnd_list,
            hwnd_edit,
            result: None,
            is_done: false,
        }));

        let raw = Rc::into_raw(inner.clone());
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw as isize);
        inner
    }
}

// ── Modal loop helper ────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn run_symbol_modal_loop(hwnd: HWND) {
    // SAFETY: Win32 FFI calls with validated arguments.
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);
        loop {
            let r = GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0);
            if r <= 0 {
                break;
            }
            if IsDialogMessageW(hwnd, &msg) == 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let rc = Rc::from_raw(ptr as *const RefCell<SymbolDialogInner>);
                let done = rc.borrow().is_done;
                let _ = Rc::into_raw(rc);
                if done {
                    break;
                }
            }
        }
    }
}

// ── SymbolPickerDialog ───────────────────────────────────────────────

/// A modal dialog that lets the user pick a single symbol from a
/// list.
///
/// Returns `Some(symbol)` on OK, `None` on cancel.
pub struct SymbolPickerDialog {
    #[cfg(target_os = "windows")]
    inner: Rc<RefCell<SymbolDialogInner>>,
    message: String,
    caption: String,
}

impl SymbolPickerDialog {
    /// Build a new symbol-picker dialog.
    ///
    /// * `message` — prompt text.
    /// * `caption` — window title.
    /// * `symbols` — the slice of symbols. The number of choices is
    ///   `symbols.len()`.
    /// * `initial` — the index of the option that's selected by
    ///   default, or `usize::MAX` for "no initial selection".
    pub fn new(
        _frame: &Frame,
        message: &str,
        caption: &str,
        symbols: &[&str],
        initial: usize,
    ) -> Self {
        let _initial_opt = if initial == usize::MAX || initial >= symbols.len() {
            None
        } else {
            Some(initial)
        };
        #[cfg(target_os = "windows")]
        {
            let inner = build_symbol_dialog(_frame, message, caption, symbols, _initial_opt);
            SymbolPickerDialog {
                inner,
                message: message.to_string(),
                caption: caption.to_string(),
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (_frame, message, caption, symbols, _initial_opt);
            SymbolPickerDialog {
                message: message.to_string(),
                caption: caption.to_string(),
            }
        }
    }

    /// Show the dialog modally. Returns the selected symbol, or
    /// `None` if the user cancelled.
    pub fn show_modal(&self) -> Option<String> {
        #[cfg(target_os = "windows")]
        {
            self.inner.borrow_mut().is_done = false;
            self.inner.borrow_mut().result = None;
            let hwnd = self.inner.borrow().hwnd;
            run_symbol_modal_loop(hwnd);
            self.inner.borrow_mut().result.take()
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = self;
            None
        }
    }

    /// Update the prompt text.
    pub fn set_message(&mut self, message: &str) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments.
        unsafe {
            let wide = to_wide(message);
            SetWindowTextW(self.inner.borrow().hwnd_label, wide.as_ptr());
        }
        self.message = message.to_string();
    }

    /// Update the dialog caption.
    pub fn set_caption(&mut self, caption: &str) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments.
        unsafe {
            let wide = to_wide(caption);
            SetWindowTextW(self.inner.borrow().hwnd, wide.as_ptr());
        }
        self.caption = caption.to_string();
    }

    /// Read the dialog message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Read the dialog caption.
    pub fn caption(&self) -> &str {
        &self.caption
    }
}

// ── Window procedure ─────────────────────────────────────────────────

#[cfg(target_os = "windows")]
unsafe extern "system" fn symbol_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as i32;
            if id == IDOK_I || id == IDCANCEL_I {
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
                if ptr != 0 {
                    let rc = Rc::from_raw(ptr as *const RefCell<SymbolDialogInner>);
                    if id == IDOK_I {
                        // Read the symbol from the EDIT first; if
                        // the user typed something, prefer that.
                        let edit_text = read_edit_text(rc.borrow().hwnd_edit);
                        if !edit_text.is_empty() {
                            rc.borrow_mut().result = Some(edit_text);
                        } else {
                            // Otherwise, read the listbox's current
                            // selection.
                            let list = rc.borrow().hwnd_list;
                            let r = SendMessageW(list, LB_GETCURSEL, 0, 0);
                            if r != -1 && r != u32::MAX as isize {
                                let len = SendMessageW(list, 0x018A, r as usize, 0) as usize; // LB_GETTEXTLEN
                                if len > 0 {
                                    let mut buf = vec![0u16; len + 1];
                                    let n = SendMessageW(
                                        list,
                                        0x0189, // LB_GETTEXT
                                        r as usize,
                                        buf.as_mut_ptr() as isize,
                                    ) as usize;
                                    let s = String::from_utf16_lossy(&buf[..n]);
                                    rc.borrow_mut().result = Some(s);
                                } else {
                                    rc.borrow_mut().result = None;
                                }
                            } else {
                                rc.borrow_mut().result = None;
                            }
                        }
                    } else {
                        rc.borrow_mut().result = None;
                    }
                    rc.borrow_mut().is_done = true;
                    let _ = Rc::into_raw(rc);
                }
                DestroyWindow(hwnd);
            }
            0
        }
        WM_KEYDOWN => {
            if wparam as u32 == VK_RETURN as u32 || wparam as u32 == VK_ESCAPE as u32 {
                let id = if wparam as u32 == VK_RETURN as u32 {
                    IDOK_I
                } else {
                    IDCANCEL_I
                };
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
                if ptr != 0 {
                    let rc = Rc::from_raw(ptr as *const RefCell<SymbolDialogInner>);
                    if id == IDOK_I {
                        let edit_text = read_edit_text(rc.borrow().hwnd_edit);
                        if !edit_text.is_empty() {
                            rc.borrow_mut().result = Some(edit_text);
                        }
                    } else {
                        rc.borrow_mut().result = None;
                    }
                    rc.borrow_mut().is_done = true;
                    let _ = Rc::into_raw(rc);
                }
                DestroyWindow(hwnd);
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_CLOSE => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let rc = Rc::from_raw(ptr as *const RefCell<SymbolDialogInner>);
                rc.borrow_mut().result = None;
                rc.borrow_mut().is_done = true;
                let _ = Rc::into_raw(rc);
            }
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let _ = Rc::from_raw(ptr as *const RefCell<SymbolDialogInner>);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[cfg(target_os = "windows")]
unsafe fn read_edit_text(hwnd: HWND) -> String {
    crate::platform::win32::read_window_text(hwnd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn types_are_constructible() {
        let _ = std::mem::size_of::<SymbolPickerDialog>();
    }
}
