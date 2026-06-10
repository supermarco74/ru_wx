//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Single- and multi-choice dialogs (`wxSingleChoiceDialog`,
//! `wxMultiChoiceDialog`).
//!
//! Both are modal popups showing a prompt, a list of choices, and
//! OK / Cancel buttons.
//!
//! | Dialog | Control | Result |
//! |---|---|---|
//! | [`SingleChoiceDialog`] | single-select `LISTBOX` | `Option<usize>` |
//! | [`MultiChoiceDialog`] | `LISTBOX` with `LBS_EXTENDEDSEL` | `Option<Vec<usize>>` |
//!
//! # Example
//! ```no_run
//! use ru_wx::prelude::*;
//!
//! let frame = Frame::builder().with_title("App").with_size(100, 100).build();
//! let dlg = SingleChoiceDialog::new(
//!     &frame,
//!     "Pick a colour:",
//!     "Colours",
//!     &["Red", "Green", "Blue"],
//!     0, // initial selection
//! );
//! if let Some(idx) = dlg.show_modal() {
//!     println!("Picked index {idx}");
//! }
//! ```

use std::cell::RefCell;
use std::rc::Rc;

use crate::window::frame::Frame;

#[cfg(target_os = "windows")]
use crate::platform::win32::{next_control_id, to_wide};
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

// ── Win32 ListBox constants ───────────────────────────────────────────

#[cfg(target_os = "windows")]
const LB_ADDSTRING: u32 = 0x0180;
#[cfg(target_os = "windows")]
const LB_SETCURSEL: u32 = 0x0186;
#[cfg(target_os = "windows")]
const LB_GETCURSEL: u32 = 0x0188;
#[cfg(target_os = "windows")]
const LB_GETSELCOUNT: u32 = 0x0190;
#[cfg(target_os = "windows")]
const LB_GETSELITEMS: u32 = 0x0191;
#[cfg(target_os = "windows")]
const LBS_NOTIFY: u32 = 0x0001;
#[cfg(target_os = "windows")]
const LBS_EXTENDEDSEL: u32 = 0x0800;
#[cfg(target_os = "windows")]
const LBN_DBLCLK: u32 = 2;

// ── Shared dialog class constants ────────────────────────────────────

const IDOK_I: i32 = 1;
const IDCANCEL_I: i32 = 2;

/// Class name registered for choice dialogs.
#[cfg(target_os = "windows")]
const CHOICE_CLASS_NAME: &str = "RuWxChoiceDialogClass";

/// Dialog width.
const DLG_W: i32 = 360;
/// Dialog height.
const DLG_H: i32 = 280;
/// Padding.
const PAD: i32 = 10;
/// Label height.
const LABEL_H: i32 = 24;
/// Listbox height.
const LIST_H: i32 = 160;
/// Button width.
const BUTTON_W: i32 = 90;
/// Button height.
const BUTTON_H: i32 = 28;

// ── Inner type ────────────────────────────────────────────────────────

/// What kind of listbox the choice dialog embeds.
#[cfg(target_os = "windows")]
#[derive(Clone, Copy)]
enum ChoiceKind {
    Single,
    Multi,
}

struct ChoiceDialogInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(target_os = "windows")]
    hwnd_label: HWND,
    #[cfg(target_os = "windows")]
    hwnd_list: HWND,
    /// `Some(index)` for single-choice, `Some(vec)` for multi-choice
    /// — set when the user clicks OK. `None` means cancel.
    result: ChoiceResult,
    /// Set by WndProc when the dialog has finished.
    is_done: bool,
    /// The list of choices, mirrored in the Rust struct so we don't
    /// have to query the listbox for the count.
    #[allow(dead_code)]
    choices_count: usize,
    #[allow(dead_code)]
    kind: ChoiceKind,
}

/// Result of a choice dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChoiceResult {
    /// User cancelled (or closed) the dialog.
    Cancelled,
    /// Single-choice: the selected index.
    Single(usize),
    /// Multi-choice: the selected indices, in selection order.
    Multi(Vec<usize>),
}

impl ChoiceResult {
    /// `true` for `Cancelled`, `false` for any user selection.
    pub fn is_cancelled(&self) -> bool {
        matches!(self, ChoiceResult::Cancelled)
    }
}

// ── Window class registration ────────────────────────────────────────

#[cfg(target_os = "windows")]
fn register_choice_class() {
    // SAFETY: Win32 FFI call with validated arguments.
    unsafe {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_name = to_wide(CHOICE_CLASS_NAME);

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(choice_wnd_proc),
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
fn build_choice_dialog(
    frame: &Frame,
    message: &str,
    caption: &str,
    choices: &[&str],
    initial: Option<usize>,
    kind: ChoiceKind,
) -> Rc<RefCell<ChoiceDialogInner>> {
    register_choice_class();

    // SAFETY: Win32 FFI calls with validated arguments.
    unsafe {
        let wide_class = to_wide(CHOICE_CLASS_NAME);
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
        let mut list_style = WS_CHILD | WS_VISIBLE | WS_BORDER | WS_VSCROLL | LBS_NOTIFY;
        if matches!(kind, ChoiceKind::Multi) {
            list_style |= LBS_EXTENDEDSEL;
        }
        let hwnd_list = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            to_wide("LISTBOX").as_ptr(),
            std::ptr::null(),
            list_style,
            PAD,
            list_y,
            DLG_W - 2 * PAD,
            LIST_H,
            hwnd,
            next_control_id() as usize as HMENU,
            hinstance,
            std::ptr::null_mut(),
        );

        // Populate the listbox.
        for (i, c) in choices.iter().enumerate() {
            let wide_c = to_wide(c);
            SendMessageW(
                hwnd_list,
                LB_ADDSTRING,
                0,
                wide_c.as_ptr() as isize,
            );
            // Set the initial selection (only for single).
            if matches!(kind, ChoiceKind::Single) {
                if let Some(idx) = initial {
                    if idx == i {
                        SendMessageW(hwnd_list, LB_SETCURSEL, i, 0);
                    }
                }
            }
        }

        // Buttons.
        let button_y = list_y + LIST_H + 16;
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
        SendMessageW(hwnd_ok, WM_SETFONT, hfont as usize, 1);
        SendMessageW(hwnd_cancel, WM_SETFONT, hfont as usize, 1);
        // Default focus on the listbox.
        SetFocus(hwnd_list);

        let inner = Rc::new(RefCell::new(ChoiceDialogInner {
            hwnd,
            hwnd_label,
            hwnd_list,
            result: ChoiceResult::Cancelled,
            is_done: false,
            choices_count: choices.len(),
            kind,
        }));

        let raw = Rc::into_raw(inner.clone());
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw as isize);
        inner
    }
}

// ── Modal loop helper ────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn run_choice_modal_loop(hwnd: HWND) {
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
                let rc = Rc::from_raw(ptr as *const RefCell<ChoiceDialogInner>);
                let done = rc.borrow().is_done;
                let _ = Rc::into_raw(rc);
                if done {
                    break;
                }
            }
        }
    }
}

// ── SingleChoiceDialog ───────────────────────────────────────────────

/// A modal dialog that lets the user pick one option from a list.
///
/// Returns `Some(index)` on OK, `None` on cancel.
pub struct SingleChoiceDialog {
    #[cfg(target_os = "windows")]
    inner: Rc<RefCell<ChoiceDialogInner>>,
    message: String,
    caption: String,
    initial: Option<usize>,
}

impl SingleChoiceDialog {
    /// Build a new single-choice dialog.
    ///
    /// * `message` — prompt text.
    /// * `caption` — window title.
    /// * `choices` — the slice of options. The number of choices is
    ///   `choices.len()`.
    /// * `initial` — the index of the option that's selected by
    ///   default, or `usize::MAX` for "no initial selection".
    pub fn new(
        _frame: &Frame,
        message: &str,
        caption: &str,
        choices: &[&str],
        initial: usize,
    ) -> Self {
        let initial_opt = if initial == usize::MAX || initial >= choices.len() {
            None
        } else {
            Some(initial)
        };
        #[cfg(target_os = "windows")]
        {
            let inner = build_choice_dialog(
                _frame,
                message,
                caption,
                choices,
                initial_opt,
                ChoiceKind::Single,
            );
            SingleChoiceDialog {
                inner,
                message: message.to_string(),
                caption: caption.to_string(),
                initial: initial_opt,
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (_frame, message, caption, choices, initial_opt);
            SingleChoiceDialog {
                message: message.to_string(),
                caption: caption.to_string(),
                initial: initial_opt,
            }
        }
    }

    /// Show the dialog modally. Returns the selected index, or `None`
    /// if the user cancelled.
    pub fn show_modal(&self) -> Option<usize> {
        #[cfg(target_os = "windows")]
        {
            self.inner.borrow_mut().is_done = false;
            self.inner.borrow_mut().result = ChoiceResult::Cancelled;
            let hwnd = self.inner.borrow().hwnd;
            run_choice_modal_loop(hwnd);
            match self.inner.borrow_mut().result.clone() {
                ChoiceResult::Single(i) => Some(i),
                _ => None,
            }
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

    /// Update the dialog caption (title).
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

// ── MultiChoiceDialog ────────────────────────────────────────────────

/// A modal dialog that lets the user pick any number of options from
/// a list (extended selection: Ctrl/Shift+Click).
///
/// Returns `Some(Vec<usize>)` on OK, `None` on cancel.
pub struct MultiChoiceDialog {
    #[cfg(target_os = "windows")]
    inner: Rc<RefCell<ChoiceDialogInner>>,
    message: String,
    caption: String,
}

impl MultiChoiceDialog {
    /// Build a new multi-choice dialog.
    pub fn new(
        _frame: &Frame,
        message: &str,
        caption: &str,
        choices: &[&str],
    ) -> Self {
        #[cfg(target_os = "windows")]
        {
            let inner = build_choice_dialog(
                _frame,
                message,
                caption,
                choices,
                None,
                ChoiceKind::Multi,
            );
            MultiChoiceDialog {
                inner,
                message: message.to_string(),
                caption: caption.to_string(),
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (_frame, message, caption, choices);
            MultiChoiceDialog {
                message: message.to_string(),
                caption: caption.to_string(),
            }
        }
    }

    /// Show the dialog modally.
    pub fn show_modal(&self) -> Option<Vec<usize>> {
        #[cfg(target_os = "windows")]
        {
            self.inner.borrow_mut().is_done = false;
            self.inner.borrow_mut().result = ChoiceResult::Cancelled;
            let hwnd = self.inner.borrow().hwnd;
            run_choice_modal_loop(hwnd);
            match self.inner.borrow_mut().result.clone() {
                ChoiceResult::Multi(v) => Some(v),
                _ => None,
            }
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

/// Read the selected indices out of the listbox HWND.
#[cfg(target_os = "windows")]
unsafe fn read_listbox_selection(hwnd: HWND) -> Vec<usize> {
    // SAFETY: Win32 FFI calls with validated arguments.
    unsafe {
        // First, see how many are selected. For a single-select
        // listbox, this returns 1.
        let count = SendMessageW(hwnd, LB_GETSELCOUNT, 0, 0) as usize;
        if count == 0 {
            return Vec::new();
        }
        let mut indices: Vec<i32> = vec![0; count];
        let n = SendMessageW(
            hwnd,
            LB_GETSELITEMS,
            count,
            indices.as_mut_ptr() as isize,
        ) as usize;
        indices.truncate(n);
        indices.into_iter().map(|i| i as usize).collect()
    }
}

/// Read the current selection index of a single-select listbox.
#[cfg(target_os = "windows")]
unsafe fn read_listbox_cur_sel(hwnd: HWND) -> Option<usize> {
    // SAFETY: Win32 FFI call with validated arguments.
    let r = unsafe { SendMessageW(hwnd, LB_GETCURSEL, 0, 0) };
    if r == -1 || r == u32::MAX as isize {
        None
    } else {
        Some(r as usize)
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn choice_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as i32;
            let code = ((wparam >> 16) & 0xFFFF) as u32;
            if id == IDOK_I || id == IDCANCEL_I {
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
                if ptr != 0 {
                    let rc = Rc::from_raw(ptr as *const RefCell<ChoiceDialogInner>);
                    if id == IDOK_I {
                        let list = rc.borrow().hwnd_list;
                        let kind = rc.borrow().kind;
                        match kind {
                            ChoiceKind::Single => {
                                if let Some(i) = read_listbox_cur_sel(list) {
                                    rc.borrow_mut().result = ChoiceResult::Single(i);
                                } else {
                                    rc.borrow_mut().result = ChoiceResult::Cancelled;
                                }
                            }
                            ChoiceKind::Multi => {
                                let v = read_listbox_selection(list);
                                if v.is_empty() {
                                    rc.borrow_mut().result = ChoiceResult::Cancelled;
                                } else {
                                    rc.borrow_mut().result = ChoiceResult::Multi(v);
                                }
                            }
                        }
                    } else {
                        rc.borrow_mut().result = ChoiceResult::Cancelled;
                    }
                    rc.borrow_mut().is_done = true;
                    let _ = Rc::into_raw(rc);
                }
                DestroyWindow(hwnd);
            } else if code == LBN_DBLCLK {
                // Double-click on a listbox row is treated as OK.
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
                if ptr != 0 {
                    let rc = Rc::from_raw(ptr as *const RefCell<ChoiceDialogInner>);
                    let list = rc.borrow().hwnd_list;
                    let kind = rc.borrow().kind;
                    match kind {
                        ChoiceKind::Single => {
                            if let Some(i) = read_listbox_cur_sel(list) {
                                rc.borrow_mut().result = ChoiceResult::Single(i);
                            }
                        }
                        ChoiceKind::Multi => {
                            let v = read_listbox_selection(list);
                            if !v.is_empty() {
                                rc.borrow_mut().result = ChoiceResult::Multi(v);
                            }
                        }
                    }
                    if !matches!(rc.borrow().result, ChoiceResult::Cancelled) {
                        rc.borrow_mut().is_done = true;
                    }
                    let _ = Rc::into_raw(rc);
                    if ptr != 0 {
                        let rc = Rc::from_raw(ptr as *const RefCell<ChoiceDialogInner>);
                        if rc.borrow().is_done {
                            let _ = Rc::into_raw(rc);
                            DestroyWindow(hwnd);
                            return 0;
                        }
                        let _ = Rc::into_raw(rc);
                    }
                }
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
                    let rc = Rc::from_raw(ptr as *const RefCell<ChoiceDialogInner>);
                    if id == IDOK_I {
                        let list = rc.borrow().hwnd_list;
                        let kind = rc.borrow().kind;
                        match kind {
                            ChoiceKind::Single => {
                                if let Some(i) = read_listbox_cur_sel(list) {
                                    rc.borrow_mut().result = ChoiceResult::Single(i);
                                }
                            }
                            ChoiceKind::Multi => {
                                let v = read_listbox_selection(list);
                                if !v.is_empty() {
                                    rc.borrow_mut().result = ChoiceResult::Multi(v);
                                }
                            }
                        }
                    } else {
                        rc.borrow_mut().result = ChoiceResult::Cancelled;
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
                let rc = Rc::from_raw(ptr as *const RefCell<ChoiceDialogInner>);
                rc.borrow_mut().result = ChoiceResult::Cancelled;
                rc.borrow_mut().is_done = true;
                let _ = Rc::into_raw(rc);
            }
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let _ = Rc::from_raw(ptr as *const RefCell<ChoiceDialogInner>);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choice_result_is_cancelled() {
        let c = ChoiceResult::Cancelled;
        assert!(c.is_cancelled());
        let s = ChoiceResult::Single(0);
        assert!(!s.is_cancelled());
        let m = ChoiceResult::Multi(vec![1, 2]);
        assert!(!m.is_cancelled());
    }

    #[test]
    fn types_are_constructible() {
        let _ = std::mem::size_of::<SingleChoiceDialog>();
        let _ = std::mem::size_of::<MultiChoiceDialog>();
    }
}
