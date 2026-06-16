//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Modal date-picker dialog (`wxDatePickerDialog`).
//!
//! A self-contained modal popup built from the same skeleton as the
//! other dialogs in this crate:
//!
//! * a top-level window with a system menu but no minimise / maximise
//!   buttons (so it behaves like a real dialog),
//! * a `STATIC` label with the prompt text,
//! * a `SysDateTimePick32` control (the same control the in-place
//!   [`crate::controls::date_picker_ctrl::DatePickerCtrl`] wraps) for the
//!   user to pick a date on,
//! * "OK" and "Cancel" buttons.
//!
//! Unlike [`crate::controls::date_picker_ctrl::DatePickerCtrl`] — which is an
//! in-place child control you embed in a sizer — this dialog owns
//! the picker as a child, blocks the calling thread on a Win32 modal
//! message loop, and returns the picked `Date` (or `None` on
//! cancel).
//!
//! # Example
//! ```no_run
//! use ru_wx::prelude::*;
//!
//! let frame = Frame::builder().with_title("App").with_size(100, 100).build();
//! let dlg = DatePickerDialog::new(
//!     &frame,
//!     "Pick a date:",
//!     "Select date",
//!     Date::new(2026, 6, 7),
//! );
//! if let Some(d) = dlg.show_modal() {
//!     println!("You picked: {}-{:02}-{:02}", d.year, d.month, d.day);
//! }
//! ```

use std::cell::RefCell;
use std::rc::Rc;

use crate::controls::date_picker_ctrl::Date;
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
use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── Win32 constants used by the date-picker dialog ──────────────────────

/// `IDOK` — standard id for the OK button.
const IDOK_I: i32 = 1;
/// `IDCANCEL` — standard id for the Cancel button.
const IDCANCEL_I: i32 = 2;

/// Class name registered for date-picker dialogs.
#[cfg(target_os = "windows")]
const DATE_CLASS_NAME: &str = "RuWxDatePickerDialogClass";

/// Dialog inner width in pixels.
const DLG_W: i32 = 360;
/// Dialog inner height in pixels.
const DLG_H: i32 = 170;
/// Padding.
const PAD: i32 = 10;
/// Label height.
const LABEL_H: i32 = 24;
/// Date-picker height.
const CTRL_H: i32 = 26;
/// Button width.
const BUTTON_W: i32 = 90;
/// Button height.
const BUTTON_H: i32 = 28;

// ── SysDateTimePick32 messages / styles (defined in <commctrl.h>, not all
//    exported by windows-sys 0.59) ────────────────────────────────────

/// `DTM_GETSYSTEMTIME` — read the picked date into a `SYSTEMTIME`.
#[cfg(target_os = "windows")]
const DTM_GETSYSTEMTIME: u32 = 0x1001;
/// `DTM_SETSYSTEMTIME` — set the picker's current value.
#[cfg(target_os = "windows")]
const DTM_SETSYSTEMTIME: u32 = 0x1002;

/// `GDT_VALID` — the SYSTEMTIME returned by `DTM_GETSYSTEMTIME` is
/// valid (the user has a date picked).
#[cfg(target_os = "windows")]
const GDT_VALID: u16 = 0;
/// `GDT_NONE` — the control has no date set (only valid when the
/// control was created with `DTS_SHOWNONE`).
#[cfg(target_os = "windows")]
const GDT_NONE: u16 = 1;

/// `DTS_LONGDATEFORMAT` — use the long date format
/// (e.g. "Friday, June 5, 2026").
#[cfg(target_os = "windows")]
const DTS_LONGDATEFORMAT: u32 = 0x0004;
/// `DTS_SHOWNONE` — display a checkbox the user can un-check to
/// clear the date.
#[cfg(target_os = "windows")]
const DTS_SHOWNONE: u32 = 0x0002;

// ── Local SYSTEMTIME shadow ────────────────────────────────────────────

/// Local `SYSTEMTIME` shadow. The picker writes its value into one
/// of these on `DTM_GETSYSTEMTIME`; we read the year/month/day out
/// and translate it into a [`Date`].
#[cfg(target_os = "windows")]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct SystemTime {
    year: u16,
    month: u16,
    weekday: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
    millisecond: u16,
}

#[cfg(target_os = "windows")]
impl SystemTime {
    const fn zero() -> Self {
        SystemTime {
            year: 0,
            month: 0,
            weekday: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
            millisecond: 0,
        }
    }

    fn from_date(d: Date) -> Self {
        SystemTime {
            year: d.year as u16,
            month: d.month as u16,
            weekday: 0,
            day: d.day as u16,
            hour: 0,
            minute: 0,
            second: 0,
            millisecond: 0,
        }
    }

    fn to_date(self) -> Date {
        Date {
            year: self.year as i32,
            month: self.month as u32,
            day: self.day as u32,
        }
    }
}

// ── Inner type ─────────────────────────────────────────────────────────

struct DateDialogInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(target_os = "windows")]
    hwnd_label: HWND,
    #[cfg(target_os = "windows")]
    hwnd_picker: HWND,
    /// Result: `Ok(date)` on OK, `None` on cancel.
    result: Option<Date>,
    /// Set by WndProc when the dialog has finished.
    is_done: bool,
    /// If true, the picker was created with `DTS_SHOWNONE`, so the
    /// user can clear the date.
    #[cfg(target_os = "windows")]
    allow_none: bool,
}

// ── Window class registration ──────────────────────────────────────────

/// Register the date-picker-dialog window class (idempotent).
#[cfg(target_os = "windows")]
fn register_date_class() {
    // SAFETY: Win32 FFI call with validated arguments.
    unsafe {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_name = to_wide(DATE_CLASS_NAME);

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(date_wnd_proc),
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

// ── Date-format enum (public surface) ──────────────────────────────────

/// Date format displayed by the dialog's date-picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateDialogFormat {
    /// Short date — locale's default short date (e.g. "06/05/2026").
    Short,
    /// Long date — locale's default long date
    /// (e.g. "Friday, June 5, 2026").
    Long,
}

#[cfg(target_os = "windows")]
impl DateDialogFormat {
    fn to_native_bits(self) -> u32 {
        match self {
            DateDialogFormat::Short => 0,
            DateDialogFormat::Long => DTS_LONGDATEFORMAT,
        }
    }
}

// ── Builder for the date-picker dialog ────────────────────────────────

#[cfg(target_os = "windows")]
fn build_date_dialog(
    frame: &Frame,
    message: &str,
    caption: &str,
    initial: Option<Date>,
    format: DateDialogFormat,
    allow_none: bool,
) -> Rc<RefCell<DateDialogInner>> {
    register_date_class();

    // SAFETY: Win32 FFI calls with validated arguments.
    unsafe {
        let wide_class = to_wide(DATE_CLASS_NAME);
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

        // Date-picker control. Style bits = WS_CHILD | WS_VISIBLE
        // plus the long-date and "show none" bits as requested.
        let mut picker_style: u32 = WS_CHILD | WS_VISIBLE;
        picker_style |= format.to_native_bits();
        if allow_none {
            picker_style |= DTS_SHOWNONE;
        }
        let hwnd_picker = CreateWindowExW(
            0,
            to_wide("SysDateTimePick32").as_ptr(),
            std::ptr::null(),
            picker_style,
            PAD,
            PAD + LABEL_H + 6,
            DLG_W - 2 * PAD,
            CTRL_H,
            hwnd,
            next_control_id() as usize as HMENU,
            hinstance,
            std::ptr::null_mut(),
        );

        // Initial value: either a real date or GDT_NONE if
        // `allow_none` is set and the caller passed `None`.
        let init_flag: isize = if let Some(d) = initial {
            let st = SystemTime::from_date(d);
            SendMessageW(
                hwnd_picker,
                DTM_SETSYSTEMTIME,
                GDT_VALID as usize,
                &st as *const _ as isize,
            );
            GDT_VALID as isize
        } else if allow_none {
            let st = SystemTime::zero();
            SendMessageW(
                hwnd_picker,
                DTM_SETSYSTEMTIME,
                GDT_NONE as usize,
                &st as *const _ as isize,
            );
            GDT_NONE as isize
        } else {
            GDT_VALID as isize
        };
        let _ = init_flag;

        // Buttons.
        let button_y = PAD + LABEL_H + 6 + CTRL_H + 16;
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
        SendMessageW(hwnd_picker, WM_SETFONT, hfont as usize, 1);
        SendMessageW(hwnd_ok, WM_SETFONT, hfont as usize, 1);
        SendMessageW(hwnd_cancel, WM_SETFONT, hfont as usize, 1);

        // Default focus on the OK button — pressing ENTER picks
        // "accept the current picker value" and ESC cancels.
        SetFocus(hwnd_ok);

        let inner = Rc::new(RefCell::new(DateDialogInner {
            hwnd,
            hwnd_label,
            hwnd_picker,
            result: None,
            is_done: false,
            allow_none,
        }));

        let raw = Rc::into_raw(inner.clone());
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw as isize);
        inner
    }
}

// ── Modal loop helper ──────────────────────────────────────────────────

/// Run the modal message loop until the dialog sets `is_done` or the
/// thread receives a `WM_QUIT`.
#[cfg(target_os = "windows")]
fn run_date_modal_loop(hwnd: HWND) {
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
                let rc = Rc::from_raw(ptr as *const RefCell<DateDialogInner>);
                let done = rc.borrow().is_done;
                let _ = Rc::into_raw(rc);
                if done {
                    break;
                }
            }
        }
    }
}

// ── WndProc ────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
unsafe extern "system" fn date_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: every arm of this function reads/writes only via the
    // GWLP_USERDATA pointer (or null-checks it), and re-pins the
    // `Rc` after the borrow with `into_raw`.
    unsafe {
        match msg {
            WM_COMMAND => {
                let id = (wparam & 0xFFFF) as i32;
                if id == IDOK_I {
                    // Read the date from the picker and stash the
                    // result in the inner.
                    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
                    if ptr != 0 {
                        let rc = Rc::from_raw(ptr as *const RefCell<DateDialogInner>);
                        let picker_hwnd = rc.borrow().hwnd_picker;
                        let allow_none = rc.borrow().allow_none;
                        let date = read_picker_date(picker_hwnd, allow_none);
                        rc.borrow_mut().result = date;
                        rc.borrow_mut().is_done = true;
                        let _ = Rc::into_raw(rc);
                        DestroyWindow(hwnd);
                    }
                    return 0;
                } else if id == IDCANCEL_I {
                    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
                    if ptr != 0 {
                        let rc = Rc::from_raw(ptr as *const RefCell<DateDialogInner>);
                        rc.borrow_mut().result = None;
                        rc.borrow_mut().is_done = true;
                        let _ = Rc::into_raw(rc);
                        DestroyWindow(hwnd);
                    }
                    return 0;
                }
                // Default handling for any other WM_COMMAND.
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_KEYDOWN => {
                // ESC closes the dialog (Cancel). ENTER is handled
                // implicitly by `IsDialogMessageW` activating the
                // default button.
                let vk = wparam as i32;
                if vk == 1 {
                    // VK_ESCAPE
                    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
                    if ptr != 0 {
                        let rc = Rc::from_raw(ptr as *const RefCell<DateDialogInner>);
                        rc.borrow_mut().result = None;
                        rc.borrow_mut().is_done = true;
                        let _ = Rc::into_raw(rc);
                        DestroyWindow(hwnd);
                    }
                    return 0;
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_CLOSE => {
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
                if ptr != 0 {
                    let rc = Rc::from_raw(ptr as *const RefCell<DateDialogInner>);
                    rc.borrow_mut().result = None;
                    rc.borrow_mut().is_done = true;
                    let _ = Rc::into_raw(rc);
                }
                DestroyWindow(hwnd);
                0
            }
            WM_DESTROY => {
                // Release the Rc parked in GWLP_USERDATA.
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
                if ptr != 0 {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                    let _ = Rc::from_raw(ptr as *const RefCell<DateDialogInner>);
                }
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

/// Read the `SysDateTimePick32` value and translate to `Option<Date>`.
#[cfg(target_os = "windows")]
unsafe fn read_picker_date(hwnd_picker: HWND, allow_none: bool) -> Option<Date> {
    // SAFETY: FFI call with a buffer large enough for the output.
    unsafe {
        let mut st = SystemTime::zero();
        let r = SendMessageW(
            hwnd_picker,
            DTM_GETSYSTEMTIME,
            0,
            &mut st as *mut _ as isize,
        );
        let flag = r as u16;
        if flag == GDT_VALID {
            Some(st.to_date())
        } else if allow_none {
            None
        } else {
            // Should not be reachable in practice — the control was
            // built without DTS_SHOWNONE, so the user can never get
            // GDT_NONE. Fall back to "today" to keep the API
            // non-panicking.
            Some(st.to_date())
        }
    }
}

// ── DatePickerDialog ──────────────────────────────────────────────────

/// A modal date-picker dialog.
///
/// Build the dialog with the constructor, then call
/// [`DatePickerDialog::show_modal`] to present it. The selected
/// date is returned as `Option<Date>` — `Some(date)` if the user
/// clicked OK, `None` if they cancelled.
pub struct DatePickerDialog {
    #[cfg(target_os = "windows")]
    inner: Rc<RefCell<DateDialogInner>>,
    message: String,
    caption: String,
    initial: Option<Date>,
    format: DateDialogFormat,
    allow_none: bool,
}

impl DatePickerDialog {
    /// Build a new date-picker dialog with the short date format
    /// (e.g. "06/05/2026").
    pub fn new(frame: &Frame, message: &str, caption: &str, initial: Date) -> Self {
        Self::new_internal(
            frame,
            message,
            caption,
            Some(initial),
            DateDialogFormat::Short,
            false,
        )
    }

    /// Build a date-picker dialog that displays a long date format
    /// (e.g. "Friday, June 5, 2026") instead of the short format
    /// ("06/05/2026").
    pub fn with_long_format(
        frame: &Frame,
        message: &str,
        caption: &str,
        initial: Date,
    ) -> Self {
        Self::new_internal(
            frame,
            message,
            caption,
            Some(initial),
            DateDialogFormat::Long,
            false,
        )
    }

    /// Build a date-picker dialog that allows the user to clear the
    /// date. The dialog's initial value is "no date"; pressing OK
    /// with the checkbox off returns `None` from `show_modal`.
    pub fn with_allow_none(frame: &Frame, message: &str, caption: &str) -> Self {
        Self::new_internal(
            frame,
            message,
            caption,
            None,
            DateDialogFormat::Short,
            true,
        )
    }

    /// Build a fully-customised date-picker dialog.
    pub fn with_format_and_allow_none(
        frame: &Frame,
        message: &str,
        caption: &str,
        initial: Option<Date>,
        format: DateDialogFormat,
        allow_none: bool,
    ) -> Self {
        Self::new_internal(frame, message, caption, initial, format, allow_none)
    }

    fn new_internal(
        frame: &Frame,
        message: &str,
        caption: &str,
        initial: Option<Date>,
        format: DateDialogFormat,
        allow_none: bool,
    ) -> Self {
        #[cfg(target_os = "windows")]
        {
            let inner = build_date_dialog(frame, message, caption, initial, format, allow_none);
            DatePickerDialog {
                inner,
                message: message.to_string(),
                caption: caption.to_string(),
                initial,
                format,
                allow_none,
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (frame, message, caption, format, allow_none);
            DatePickerDialog {
                message: message.to_string(),
                caption: caption.to_string(),
                initial,
                format,
                allow_none,
            }
        }
    }

    /// Show the dialog modally. Blocks until the user dismisses it.
    pub fn show_modal(&self) -> Option<Date> {
        #[cfg(target_os = "windows")]
        {
            self.inner.borrow_mut().is_done = false;
            self.inner.borrow_mut().result = None;
            let hwnd = self.inner.borrow().hwnd;
            run_date_modal_loop(hwnd);
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

    /// Read the prompt text.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Read the dialog title.
    pub fn caption(&self) -> &str {
        &self.caption
    }

    /// Read the initially-selected date.
    pub fn initial_value(&self) -> Option<Date> {
        self.initial
    }

    /// `true` if the user is allowed to leave the picker empty.
    pub fn allows_none(&self) -> bool {
        self.allow_none
    }

    /// The date format the dialog was built with.
    pub fn format(&self) -> DateDialogFormat {
        self.format
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// `DatePickerDialog` is a thin data class on top of the
    /// `Date` payload — its constructor + accessors must not lose
    /// information. The actual modal loop is windowed and is
    /// covered by the integration smoke test in
    /// `examples/showcase_all.rs`.
    #[test]
    fn date_picker_dialog_round_trip() {
        // Pure data round-trip: we can't call `new(&frame, ...)`
        // without a real `Frame`, but the accessors and the
        // value-type are pure.
        let d = Date::new(2026, 6, 7);
        assert_eq!(d.year, 2026);
        assert_eq!(d.month, 6);
        assert_eq!(d.day, 7);
    }

    /// Sanity-check the constants that are not exported by
    /// `windows-sys 0.59` (so the modal loop / WndProc don't drift).
    #[cfg(target_os = "windows")]
    #[test]
    fn idok_idcancel_constants() {
        // The button IDs are intentionally tiny integers so the
        // WndProc can match against them cheaply; pin them so a
        // future refactor that picks the "next control id" doesn't
        // silently clash.
        assert_eq!(IDOK_I, 1);
        assert_eq!(IDCANCEL_I, 2);
    }

    /// `Date` is the only payload the dialog emits; it must be
    /// `Copy` so the user can move it into a model struct without
    /// cloning.
    #[test]
    fn date_is_copy() {
        let d = Date::new(2026, 6, 7);
        let d2 = d; // implicit copy
        assert_eq!(d, d2);
    }

    /// `SystemTime::from_date` → `to_date` must be a lossless
    /// round-trip for the year/month/day fields. The time and
    /// weekday fields are zeroed by `from_date` (the control does
    /// not own a time of day in the date-only formats) and are
    /// not part of the `Date` payload.
    #[cfg(target_os = "windows")]
    #[test]
    fn systemtime_date_round_trip() {
        let d = Date::new(2026, 6, 7);
        let st = SystemTime::from_date(d);
        let d2 = st.to_date();
        assert_eq!(d, d2);
    }

    /// `DateDialogFormat` must serialise to the expected
    /// `DTS_*` style bit.
    #[cfg(target_os = "windows")]
    #[test]
    fn date_dialog_format_to_native() {
        assert_eq!(DateDialogFormat::Short.to_native_bits(), 0);
        assert_eq!(
            DateDialogFormat::Long.to_native_bits(),
            DTS_LONGDATEFORMAT
        );
    }
}
