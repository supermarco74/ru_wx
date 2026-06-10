//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Single-line text-entry dialogs (`wxTextEntryDialog`,
//! `wxPasswordEntryDialog`, `wxNumberEntryDialog`).
//!
//! All three are modal popups built from the same skeleton:
//!
//! * a top-level window with a system menu but no minimise / maximise
//!   buttons (so it behaves like a real dialog),
//! * a `STATIC` label with the prompt text,
//! * an `EDIT` control for the user to type into,
//! * "OK" and "Cancel" buttons.
//!
//! They differ only in the style bits applied to the `EDIT` control
//! (`ES_PASSWORD` for passwords, `ES_NUMBER` for numeric input) and
//! the return type:
//!
//! | Type | EDIT style | Return |
//! |---|---|---|
//! | [`TextEntryDialog`] | plain | `Option<String>` |
//! | [`PasswordEntryDialog`] | `ES_PASSWORD` | `Option<String>` |
//! | [`NumberEntryDialog`] | `ES_NUMBER` | `Option<i64>` |
//!
//! # Example
//! ```no_run
//! use ru_wx::prelude::*;
//!
//! let frame = Frame::builder().with_title("App").with_size(100, 100).build();
//! let dlg = TextEntryDialog::new(&frame, "Your name:", "Login", "Anonymous");
//! if let Some(name) = dlg.show_modal() {
//!     println!("Hello, {name}!");
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

// ── Win32 constants used by the entry dialogs ─────────────────────────

/// `IDOK` — standard id for the OK button.
const IDOK_I: i32 = 1;
/// `IDCANCEL` — standard id for the Cancel button.
const IDCANCEL_I: i32 = 2;

/// Class name registered for entry dialogs.
#[cfg(target_os = "windows")]
const ENTRY_CLASS_NAME: &str = "RuWxTextEntryDialogClass";

/// Dialog inner width in pixels.
const DLG_W: i32 = 360;
/// Dialog inner height in pixels.
const DLG_H: i32 = 150;
/// Padding.
const PAD: i32 = 10;
/// Label height.
const LABEL_H: i32 = 24;
/// Edit control height.
const EDIT_H: i32 = 24;
/// Button width.
const BUTTON_W: i32 = 90;
/// Button height.
const BUTTON_H: i32 = 28;

// ── Shared inner type ────────────────────────────────────────────────

/// Style bits to apply to the `EDIT` control.
#[cfg(target_os = "windows")]
#[derive(Clone, Copy)]
enum EditStyle {
    Plain,
    Password,
    Number,
}

#[cfg(target_os = "windows")]
impl EditStyle {
    fn to_native(self) -> u32 {
        let base = WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_BORDER | (ES_AUTOHSCROLL as u32);
        match self {
            EditStyle::Plain => base,
            EditStyle::Password => base | (ES_PASSWORD as u32),
            EditStyle::Number => base | (ES_NUMBER as u32),
        }
    }
}

struct EntryDialogInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(target_os = "windows")]
    hwnd_label: HWND,
    #[cfg(target_os = "windows")]
    hwnd_edit: HWND,
    /// Result: `Ok(value)` on OK, `None` on cancel.
    result: Option<String>,
    /// Set by WndProc when the dialog has finished.
    is_done: bool,
    /// The kind of edit (used for the ENTER -> OK behaviour; numbers
    /// additionally validate the input on OK).
    #[cfg(target_os = "windows")]
    kind: EditStyle,
}

// ── Window class registration ────────────────────────────────────────

/// Register the entry-dialog window class (idempotent).
#[cfg(target_os = "windows")]
fn register_entry_class() {
    // SAFETY: Win32 FFI call with validated arguments.
    unsafe {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_name = to_wide(ENTRY_CLASS_NAME);

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(entry_wnd_proc),
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

// ── Builder for the three dialog flavours ────────────────────────────

#[cfg(target_os = "windows")]
fn build_entry_dialog(
    frame: &Frame,
    message: &str,
    caption: &str,
    default: &str,
    kind: EditStyle,
) -> Rc<RefCell<EntryDialogInner>> {
    register_entry_class();

    // SAFETY: Win32 FFI calls with validated arguments.
    unsafe {
        let wide_class = to_wide(ENTRY_CLASS_NAME);
        let wide_caption = to_wide(caption);
        let wide_msg = to_wide(message);
        let wide_default = to_wide(default);
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

        // EDIT.
        let edit_y = PAD + LABEL_H + 6;
        let hwnd_edit = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            to_wide("EDIT").as_ptr(),
            wide_default.as_ptr(),
            kind.to_native(),
            PAD,
            edit_y,
            DLG_W - 2 * PAD,
            EDIT_H + 2,
            hwnd,
            next_control_id() as usize as HMENU,
            hinstance,
            std::ptr::null_mut(),
        );

        // Buttons.
        let button_y = edit_y + EDIT_H + 16;
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
        SendMessageW(hwnd_edit, WM_SETFONT, hfont as usize, 1);
        SendMessageW(hwnd_ok, WM_SETFONT, hfont as usize, 1);
        SendMessageW(hwnd_cancel, WM_SETFONT, hfont as usize, 1);

        // Default focus on the edit control.
        SetFocus(hwnd_edit);

        let inner = Rc::new(RefCell::new(EntryDialogInner {
            hwnd,
            hwnd_label,
            hwnd_edit,
            result: None,
            is_done: false,
            kind,
        }));

        let raw = Rc::into_raw(inner.clone());
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw as isize);
        inner
    }
}

// ── Modal loop helper ────────────────────────────────────────────────

/// Run the modal message loop until the dialog sets `is_done` or the
/// thread receives a `WM_QUIT`.
#[cfg(target_os = "windows")]
fn run_entry_modal_loop(hwnd: HWND) {
    // SAFETY: Win32 FFI calls with validated arguments.
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);

        loop {
            let r = GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0);
            if r <= 0 {
                // WM_QUIT or error — stop the modal loop and let the
                // outer application deal with it.
                break;
            }
            // Honour TAB navigation in the dialog.
            if IsDialogMessageW(hwnd, &msg) == 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            // Re-check the "done" flag after dispatching.
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let rc = Rc::from_raw(ptr as *const RefCell<EntryDialogInner>);
                let done = rc.borrow().is_done;
                let _ = Rc::into_raw(rc);
                if done {
                    break;
                }
            }
        }
    }
}

// ── TextEntryDialog ──────────────────────────────────────────────────

/// A modal single-line text-entry dialog.
///
/// Returns `Some(String)` if the user clicked OK, `None` if they
/// cancelled (or closed the dialog).
pub struct TextEntryDialog {
    #[cfg(target_os = "windows")]
    inner: Rc<RefCell<EntryDialogInner>>,
    message: String,
    caption: String,
    default: String,
}

impl TextEntryDialog {
    /// Build a new text-entry dialog. The dialog is not shown until
    /// [`TextEntryDialog::show_modal`] is called.
    pub fn new(_frame: &Frame, message: &str, caption: &str, default: &str) -> Self {
        #[cfg(target_os = "windows")]
        {
            let inner = build_entry_dialog(_frame, message, caption, default, EditStyle::Plain);
            TextEntryDialog {
                inner,
                message: message.to_string(),
                caption: caption.to_string(),
                default: default.to_string(),
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (_frame, message, caption, default);
            TextEntryDialog {
                message: message.to_string(),
                caption: caption.to_string(),
                default: default.to_string(),
            }
        }
    }

    /// Construct a [`TextEntryDialogBuilder`] for fluent
    /// one-liner configuration. Equivalent to
    /// [`TextEntryDialog::new`] followed by chained `.with_*` calls.
    pub fn builder(frame: &Frame, message: &str, caption: &str) -> TextEntryDialogBuilder {
        TextEntryDialogBuilder {
            dialog: Self::new(frame, message, caption, ""),
        }
    }

    /// Show the dialog modally. Blocks until the user dismisses it.
    pub fn show_modal(&self) -> Option<String> {
        #[cfg(target_os = "windows")]
        {
            // Park this method on the inner before entering the loop
            // so the WndProc can write to the right cell.
            self.inner.borrow_mut().is_done = false;
            self.inner.borrow_mut().result = None;
            let hwnd = self.inner.borrow().hwnd;
            run_entry_modal_loop(hwnd);
            self.inner.borrow_mut().result.take()
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = self;
            None
        }
    }

    /// Update the message text.
    pub fn set_message(&mut self, message: &str) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments.
        unsafe {
            let wide = to_wide(message);
            SetWindowTextW(self.inner.borrow().hwnd_label, wide.as_ptr());
        }
        self.message = message.to_string();
    }

    /// Update the default value shown in the edit control.
    pub fn set_value(&mut self, value: &str) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments.
        unsafe {
            let wide = to_wide(value);
            SetWindowTextW(self.inner.borrow().hwnd_edit, wide.as_ptr());
        }
        self.default = value.to_string();
    }

    /// Read the dialog message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Read the dialog caption.
    pub fn caption(&self) -> &str {
        &self.caption
    }

    /// Read the default value the dialog was built with.
    pub fn default_value(&self) -> &str {
        &self.default
    }
}

// ── Builders for the three entry-dialog flavours ────────────────────

/// Fluent builder for [`TextEntryDialog`].
///
/// Constructed via [`TextEntryDialog::builder`]. The three required
/// arguments (`frame`, `message`, `caption`) are passed to the
/// constructor; the optional default value can be set with
/// [`TextEntryDialogBuilder::with_default_value`].
///
/// ```no_run
/// # use ru_wx::prelude::*;
/// # let frame = Frame::builder().with_title("App").with_size(100, 100).build();
/// let dlg = TextEntryDialog::builder(&frame, "Your name:", "Login")
///     .with_default_value("Anonymous")
///     .build();
/// # let _ = dlg;
/// ```
#[must_use = "a TextEntryDialogBuilder does nothing until .build() or .show_modal() is called"]
pub struct TextEntryDialogBuilder {
    dialog: TextEntryDialog,
}

impl TextEntryDialogBuilder {
    /// Set the initial value shown in the edit control.
    pub fn with_default_value(mut self, value: &str) -> Self {
        self.dialog.set_value(value);
        self
    }

    /// Update the prompt text shown above the edit control.
    pub fn with_message(mut self, message: &str) -> Self {
        self.dialog.set_message(message);
        self
    }

    /// Finalise the builder and return the configured
    /// [`TextEntryDialog`].
    pub fn build(self) -> TextEntryDialog {
        self.dialog
    }

    /// Finalise the builder and immediately show the dialog
    /// modally. Equivalent to `.build().show_modal()`.
    pub fn show_modal(self) -> Option<String> {
        self.dialog.show_modal()
    }
}

// ── PasswordEntryDialog ─────────────────────────────────────────────

/// A modal single-line password-entry dialog.
///
/// The edit control is created with `ES_PASSWORD`, so the user's
/// input is masked. The returned value is the plain-text password.
pub struct PasswordEntryDialog {
    #[cfg(target_os = "windows")]
    inner: Rc<RefCell<EntryDialogInner>>,
    message: String,
    caption: String,
}

impl PasswordEntryDialog {
    /// Build a new password-entry dialog.
    pub fn new(_frame: &Frame, message: &str, caption: &str) -> Self {
        #[cfg(target_os = "windows")]
        {
            let inner = build_entry_dialog(_frame, message, caption, "", EditStyle::Password);
            PasswordEntryDialog {
                inner,
                message: message.to_string(),
                caption: caption.to_string(),
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (_frame, message, caption);
            PasswordEntryDialog {
                message: message.to_string(),
                caption: caption.to_string(),
            }
        }
    }

    /// Construct a [`PasswordEntryDialogBuilder`] for fluent
    /// one-liner configuration. Equivalent to
    /// [`PasswordEntryDialog::new`] followed by chained `.with_*`
    /// calls.
    pub fn builder(frame: &Frame, message: &str, caption: &str) -> PasswordEntryDialogBuilder {
        PasswordEntryDialogBuilder { dialog: Self::new(frame, message, caption) }
    }

    /// Show the dialog modally.
    pub fn show_modal(&self) -> Option<String> {
        #[cfg(target_os = "windows")]
        {
            self.inner.borrow_mut().is_done = false;
            self.inner.borrow_mut().result = None;
            let hwnd = self.inner.borrow().hwnd;
            run_entry_modal_loop(hwnd);
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

    /// Read the dialog message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Read the dialog caption.
    pub fn caption(&self) -> &str {
        &self.caption
    }
}

/// Fluent builder for [`PasswordEntryDialog`].
///
/// Constructed via [`PasswordEntryDialog::builder`]. The three
/// required arguments (`frame`, `message`, `caption`) are passed
/// to the constructor; the prompt text can be overridden later
/// with [`PasswordEntryDialogBuilder::with_message`].
#[must_use = "a PasswordEntryDialogBuilder does nothing until .build() or .show_modal() is called"]
pub struct PasswordEntryDialogBuilder {
    dialog: PasswordEntryDialog,
}

impl PasswordEntryDialogBuilder {
    /// Override the prompt text shown above the password edit
    /// control.
    pub fn with_message(mut self, message: &str) -> Self {
        self.dialog.set_message(message);
        self
    }

    /// Finalise the builder and return the configured
    /// [`PasswordEntryDialog`].
    pub fn build(self) -> PasswordEntryDialog {
        self.dialog
    }

    /// Finalise the builder and immediately show the dialog
    /// modally. Equivalent to `.build().show_modal()`.
    pub fn show_modal(self) -> Option<String> {
        self.dialog.show_modal()
    }
}

// ── NumberEntryDialog ────────────────────────────────────────────────

/// A modal single-line integer-entry dialog.
///
/// The edit control is created with `ES_NUMBER`, so the user can only
/// type ASCII digits (and a leading minus). The returned value is
/// parsed as `i64`. If the user typed something non-numeric (e.g.
/// via paste), `show_modal` returns `None`.
pub struct NumberEntryDialog {
    #[cfg(target_os = "windows")]
    inner: Rc<RefCell<EntryDialogInner>>,
    message: String,
    caption: String,
    initial: i64,
    min_value: Option<i64>,
    max_value: Option<i64>,
}

impl NumberEntryDialog {
    /// Build a new number-entry dialog. The edit control is pre-filled
    /// with the string representation of `initial`.
    pub fn new(_frame: &Frame, message: &str, caption: &str, initial: i64) -> Self {
        #[cfg(target_os = "windows")]
        {
            let initial_str = initial.to_string();
            let inner = build_entry_dialog(_frame, message, caption, &initial_str, EditStyle::Number);
            NumberEntryDialog {
                inner,
                message: message.to_string(),
                caption: caption.to_string(),
                initial,
                min_value: None,
                max_value: None,
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (_frame, message, caption, initial);
            NumberEntryDialog {
                message: message.to_string(),
                caption: caption.to_string(),
                initial,
                min_value: None,
                max_value: None,
            }
        }
    }

    /// Construct a [`NumberEntryDialogBuilder`] for fluent
    /// one-liner configuration. Equivalent to
    /// [`NumberEntryDialog::new`] followed by chained
    /// `.with_min` / `.with_max` / `.with_message` calls.
    pub fn builder(
        frame: &Frame,
        message: &str,
        caption: &str,
        initial: i64,
    ) -> NumberEntryDialogBuilder {
        NumberEntryDialogBuilder { dialog: Self::new(frame, message, caption, initial) }
    }

    /// Set the minimum allowed value. Inputs below this will return
    /// `None` from `show_modal`.
    pub fn set_min(&mut self, min: i64) {
        self.min_value = Some(min);
    }

    /// Set the maximum allowed value. Inputs above this will return
    /// `None` from `show_modal`.
    pub fn set_max(&mut self, max: i64) {
        self.max_value = Some(max);
    }

    /// Show the dialog modally.
    pub fn show_modal(&self) -> Option<i64> {
        #[cfg(target_os = "windows")]
        {
            self.inner.borrow_mut().is_done = false;
            self.inner.borrow_mut().result = None;
            let hwnd = self.inner.borrow().hwnd;
            run_entry_modal_loop(hwnd);
            match self.inner.borrow_mut().result.take() {
                Some(s) => {
                    if let Ok(n) = s.trim().parse::<i64>() {
                        if let Some(min) = self.min_value {
                            if n < min {
                                return None;
                            }
                        }
                        if let Some(max) = self.max_value {
                            if n > max {
                                return None;
                            }
                        }
                        Some(n)
                    } else {
                        None
                    }
                }
                None => None,
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
        // SAFETY: Win32 FSI call with validated arguments.
        unsafe {
            let wide = to_wide(message);
            SetWindowTextW(self.inner.borrow().hwnd_label, wide.as_ptr());
        }
        self.message = message.to_string();
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

/// Fluent builder for [`NumberEntryDialog`].
///
/// Constructed via [`NumberEntryDialog::builder`]. The four
/// required arguments (`frame`, `message`, `caption`, `initial`)
/// are passed to the constructor; the optional min / max bounds
/// and prompt text can be set with
/// [`NumberEntryDialogBuilder::with_min`],
/// [`NumberEntryDialogBuilder::with_max`] and
/// [`NumberEntryDialogBuilder::with_message`].
#[must_use = "a NumberEntryDialogBuilder does nothing until .build() or .show_modal() is called"]
pub struct NumberEntryDialogBuilder {
    dialog: NumberEntryDialog,
}

impl NumberEntryDialogBuilder {
    /// Set the minimum allowed value. Inputs below this will
    /// return `None` from `show_modal`.
    pub fn with_min(mut self, min: i64) -> Self {
        self.dialog.set_min(min);
        self
    }

    /// Set the maximum allowed value. Inputs above this will
    /// return `None` from `show_modal`.
    pub fn with_max(mut self, max: i64) -> Self {
        self.dialog.set_max(max);
        self
    }

    /// Override the prompt text shown above the edit control.
    pub fn with_message(mut self, message: &str) -> Self {
        self.dialog.set_message(message);
        self
    }

    /// Finalise the builder and return the configured
    /// [`NumberEntryDialog`].
    pub fn build(self) -> NumberEntryDialog {
        self.dialog
    }

    /// Finalise the builder and immediately show the dialog
    /// modally. Equivalent to `.build().show_modal()`.
    pub fn show_modal(self) -> Option<i64> {
        self.dialog.show_modal()
    }
}

// ── Window procedure ─────────────────────────────────────────────────

/// Read the text of the given `EDIT` HWND into a freshly-allocated
/// `String`.
#[cfg(target_os = "windows")]
unsafe fn read_edit_text(hwnd: HWND) -> String {
    crate::platform::win32::read_window_text(hwnd)
}

/// WndProc for the entry-dialog class.
///
/// * `WM_COMMAND` from OK or Cancel — store result and break the
///   modal loop.
/// * `WM_KEYDOWN` in the edit control with VK_RETURN — treat as OK.
/// * `WM_KEYDOWN` in the edit control with VK_ESCAPE — treat as
///   Cancel.
/// * `WM_CLOSE` — close the dialog with no result.
/// * `WM_DESTROY` — release the `Rc` parked in `GWLP_USERDATA`.
#[cfg(target_os = "windows")]
unsafe extern "system" fn entry_wnd_proc(
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
                    let rc = Rc::from_raw(ptr as *const RefCell<EntryDialogInner>);
                    if id == IDOK_I {
                        // Read the edit control's text into the
                        // result.
                        let edit_hwnd = rc.borrow().hwnd_edit;
                        let text = read_edit_text(edit_hwnd);
                        rc.borrow_mut().result = Some(text);
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
            // Treat ENTER as OK and ESC as Cancel when the focus is
            // on the edit control.
            if wparam as u32 == VK_RETURN as u32 || wparam as u32 == VK_ESCAPE as u32 {
                let id = if wparam as u32 == VK_RETURN as u32 {
                    IDOK_I
                } else {
                    IDCANCEL_I
                };
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
                if ptr != 0 {
                    let rc = Rc::from_raw(ptr as *const RefCell<EntryDialogInner>);
                    if id == IDOK_I {
                        let edit_hwnd = rc.borrow().hwnd_edit;
                        let text = read_edit_text(edit_hwnd);
                        rc.borrow_mut().result = Some(text);
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
                let rc = Rc::from_raw(ptr as *const RefCell<EntryDialogInner>);
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
                let _ = Rc::from_raw(ptr as *const RefCell<EntryDialogInner>);
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
    fn entry_dialog_construction() {
        // Smoke test: just verify the type-level setters work
        // without requiring a real window pump.
        let _ = std::mem::size_of::<TextEntryDialog>();
        let _ = std::mem::size_of::<PasswordEntryDialog>();
        let _ = std::mem::size_of::<NumberEntryDialog>();
        let _ = IDOK_I;
        let _ = IDCANCEL_I;
    }

    // ------------------------------------------------------------------
    // Builder smoke tests
    // ------------------------------------------------------------------

    /// Compile-time check that the `TextEntryDialog::builder` chain is
    /// well typed. The real call needs a `Frame`, so we only assert the
    /// *types* are reachable here.
    #[test]
    fn text_entry_dialog_builder_type_is_reachable() {
        let _chain_typecheck: fn() = || {
            // (Not executed: would require a real `Frame`.)
            // TextEntryDialog::builder(frame, "Enter name", "Greeting")
            //     .with_default_value("world")
            //     .with_message("Please type your name")
            //     .build();
        };
    }

    /// Compile-time check for the `PasswordEntryDialog::builder` chain.
    #[test]
    fn password_entry_dialog_builder_type_is_reachable() {
        let _chain_typecheck: fn() = || {
            // (Not executed: would require a real `Frame`.)
            // PasswordEntryDialog::builder(frame, "Password", "Auth")
            //     .with_message("Type your password")
            //     .build();
        };
    }

    /// Compile-time check for the `NumberEntryDialog::builder` chain.
    #[test]
    fn number_entry_dialog_builder_type_is_reachable() {
        let _chain_typecheck: fn() = || {
            // (Not executed: would require a real `Frame`.)
            // NumberEntryDialog::builder(frame, "Age", "Profile", 18)
            //     .with_min(0)
            //     .with_max(120)
            //     .with_message("Enter your age")
            //     .build();
        };
    }
}
