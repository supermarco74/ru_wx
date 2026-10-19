//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! wxDatePickerCtrl — a date picker with a calendar drop-down.
//!
//! On Windows this wraps the standard `SysDateTimePick32` common control.
//! The control stores its value in a `SYSTEMTIME` struct. The date can be
//! `None` to represent "no date selected" if the control was created
//! with [`DatePickerCtrl::allow_none`].
//!
//! Use [`DatePickerCtrl::on_date_change`] to register a callback that
//! fires when the user picks a different date.

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};
use crate::window::frame::Frame;

use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Controls::NMHDR;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── Win32 SysDateTimePick32 constants ───────────────────────────────────

// `pub(crate)`: `time_picker_ctrl.rs` riusa le stesse costanti Win32
// (classe `SysDateTimePick32` condivisa) — definizione unica in mezzo.
#[cfg(target_os = "windows")]
pub(crate) const DTM_GETSYSTEMTIME: u32 = 0x1001;
#[cfg(target_os = "windows")]
pub(crate) const DTM_SETSYSTEMTIME: u32 = 0x1002;

/// DTS_UPDOWN — display spin buttons instead of a calendar drop-down.
#[cfg(target_os = "windows")]
pub(crate) const DTS_UPDOWN: u32 = 0x0001;
/// DTS_SHOWNONE — display a checkbox; user can un-check to set "no date".
#[cfg(target_os = "windows")]
const DTS_SHOWNONE: u32 = 0x0002;
/// DTS_LONGDATEFORMAT — use the long date format.
#[cfg(target_os = "windows")]
const DTS_LONGDATEFORMAT: u32 = 0x0004;
/// DTS_TIMEFORMAT — display time as well as date.
#[cfg(target_os = "windows")]
pub(crate) const DTS_TIMEFORMAT: u32 = 0x0009;

/// GDT_VALID — the SYSTEMTIME returned is valid.
#[cfg(target_os = "windows")]
pub(crate) const GDT_VALID: u16 = 0;
/// GDT_NONE — the control has no date set (only valid with DTS_SHOWNONE).
#[cfg(target_os = "windows")]
pub(crate) const GDT_NONE: u16 = 1;

/// `DTN_DATETIMECHANGE` — the SysDateTimePick32 notification code
/// delivered via `WM_NOTIFY` whenever the user picks a different
/// date (or clears the control with `DTS_SHOWNONE`). The
/// notification body is a `tagNMDATETIMECHANGE` struct
/// (`NMHDR + dwFlags + SYSTEMTIME`).
///
/// `pub(crate)` rather than `pub` because this is a Win32
/// constant internal to the library's notification dispatch
/// (the frame's `WM_NOTIFY` arm checks against this value to
/// route the notification to the `dtn_handlers` map; see
/// `src/frame.rs`). User code should not need this constant
/// — the user-facing surface is
/// [`DatePickerCtrl::on_date_change`], which takes a
/// `FnMut(Option<Date>)` and hides the constant behind the
/// callback signature.
#[cfg(target_os = "windows")]
pub(crate) const DTN_DATETIMECHANGE: u32 = 0xFFFFFD09_u32;

// ── Public date type ──────────────────────────────────────────────────

/// A simple calendar date used by [`DatePickerCtrl`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    pub year: i32,
    pub month: u32, // 1..=12
    pub day: u32,   // 1..=31
}

impl Date {
    /// Create a new date. The caller is responsible for ensuring that
    /// `month` is in 1..=12 and `day` is in 1..=31.
    pub fn new(year: i32, month: u32, day: u32) -> Self {
        Self { year, month, day }
    }
}

// `pub(crate)`: sono strutture ABI Win32 condivise con
// `time_picker_ctrl.rs` (stesso controllo `SysDateTimePick32`,
// stesso layout `SYSTEMTIME` / `NMDATETIMECHANGE`).
#[cfg(target_os = "windows")]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct SystemTime {
    pub(crate) year: u16,
    pub(crate) month: u16,
    pub(crate) weekday: u16,
    pub(crate) day: u16,
    pub(crate) hour: u16,
    pub(crate) minute: u16,
    pub(crate) second: u16,
    pub(crate) millisecond: u16,
}

#[cfg(target_os = "windows")]
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct NmDateTimeChange {
    pub(crate) nmhdr: NMHDR,
    pub(crate) dw_flags: u32,
    pub(crate) st: SystemTime,
}

// The NMDATETIMECHANGE struct is a Win32 ABI struct: the
// `#[repr(C)]` attribute gives it the same field layout as the
// C declaration in `<commctrl.h>`. The `NMHDR` is the
// notification header (carrying `code` / `idFrom` / `hwndFrom`),
// `dw_flags` is `GDT_VALID` (0) or `GDT_NONE` (1), and `st` is
// the new date as a `SYSTEMTIME` (the time fields are zeroed by
// the control if the date-only format was used).
#[cfg(target_os = "windows")]
impl NmDateTimeChange {
    /// Extract the new value as a `Date` if `dw_flags ==
    /// GDT_VALID`, or `None` if the user cleared the control
    /// (only possible with `DTS_SHOWNONE`).
    fn to_option(self) -> Option<Date> {
        if self.dw_flags as u16 == GDT_VALID {
            Some(self.st.to_date())
        } else {
            None
        }
    }
}

#[cfg(target_os = "windows")]
impl SystemTime {
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

struct DatePickerCtrlInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(target_os = "macos")]
    stub_handle: crate::platform::NativeHandle,
    #[cfg(target_os = "macos")]
    parent_handle: crate::platform::NativeHandle,
    id: u16,
    rect: Rect,
    enabled: bool,
    visible: bool,
}

#[derive(Clone)]
pub struct DatePickerCtrl {
    inner: Rc<RefCell<DatePickerCtrlInner>>,
}

/// Date format for the control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateFormat {
    /// Short date format — locale's default short date (e.g. "06/05/2026").
    Short,
    /// Long date format — locale's default long date (e.g. "Friday, June 5, 2026").
    Long,
    /// Show time as well as date in locale's default time format.
    Time,
}

impl DatePickerCtrl {
    /// Create a new date-picker control as a child of `parent`.
    ///
    /// By default, uses the short date format and does not allow
    /// "no date".
    pub fn new<W: Window>(parent_in: &W) -> Self {
        Self::new_with_format(parent_in, DateFormat::Short, false)
    }

    /// Create a new date-picker with the chosen date format.
    pub fn new_with_format<W: Window>(parent_in: &W, format: DateFormat, allow_none: bool) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        let style = {
            let mut s: u32 = WS_CHILD | WS_VISIBLE;
            s |= match format {
                DateFormat::Short => 0,
                DateFormat::Long => DTS_LONGDATEFORMAT,
                DateFormat::Time => DTS_TIMEFORMAT,
            };
            if allow_none {
                s |= DTS_SHOWNONE;
            }
            s
        };

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = crate::platform::window_handle_as_hwnd(parent_in);
            let wide_class = to_wide("SysDateTimePick32");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                style,
                0,
                0,
                160,
                24,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = (parent_in, format, allow_none);

        #[cfg(target_os = "macos")]
        let parent_handle = crate::platform::window_handle(parent_in);
        #[cfg(target_os = "macos")]
        let stub_handle = crate::platform::appkit::create_date_picker(
            parent_handle,
            id,
            Rect::new(0, 0, 160, 24),
        );
        let _ = (format, allow_none);

        DatePickerCtrl {
            inner: Rc::new(RefCell::new(DatePickerCtrlInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(target_os = "macos")]
                stub_handle,
                #[cfg(target_os = "macos")]
                parent_handle,
                id,
                rect: Rect::new(0, 0, 160, 24),
                enabled: true,
                visible: true,
            })),
        }
    }

    /// Create a date-picker that uses spin buttons instead of a calendar
    /// drop-down.
    pub fn new_spin<W: Window>(parent_in: &W) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = crate::platform::window_handle_as_hwnd(parent_in);
            let wide_class = to_wide("SysDateTimePick32");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | DTS_UPDOWN,
                0,
                0,
                160,
                24,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = parent_in;

        #[cfg(target_os = "macos")]
        let parent_handle = crate::platform::window_handle(parent_in);
        #[cfg(target_os = "macos")]
        let stub_handle = crate::platform::appkit::create_date_picker(
            parent_handle,
            id,
            Rect::new(0, 0, 160, 24),
        );

        DatePickerCtrl {
            inner: Rc::new(RefCell::new(DatePickerCtrlInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(target_os = "macos")]
                stub_handle,
                #[cfg(target_os = "macos")]
                parent_handle,
                id,
                rect: Rect::new(0, 0, 160, 24),
                enabled: true,
                visible: true,
            })),
        }
    }

    /// Create a date-picker that allows "no date" (DTS_SHOWNONE).
    pub fn allow_none<W: Window>(parent_in: &W) -> Self {
        Self::new_with_format(parent_in, DateFormat::Short, true)
    }

    /// Return the current date, or `None` if the control has no date
    /// (only possible if the control was created with `allow_none`).
    pub fn get_value(&self) -> Option<Date> {
        #[cfg(target_os = "windows")]
        {
            let mut st = SystemTime {
                year: 0,
                month: 0,
                weekday: 0,
                day: 0,
                hour: 0,
                minute: 0,
                second: 0,
                millisecond: 0,
            };
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let result = unsafe {
                SendMessageW(
                    self.inner.borrow().hwnd,
                    DTM_GETSYSTEMTIME,
                    0,
                    &mut st as *mut _ as isize,
                )
            };
            let flag = result as u16;
            if flag == GDT_VALID {
                Some(st.to_date())
            } else {
                None
            }
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::date_picker_get_value(self.inner.borrow().stub_handle)
                .map(|(y, m, d)| Date::new(y, m, d))
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            None
        }
    }

    /// Set the current date. If the control was created without
    /// `allow_none` and `value` is `None`, the call is a no-op.
    pub fn set_value(&self, value: Option<Date>) {
        #[cfg(target_os = "windows")]
        {
            let (flag, st) = match value {
                Some(d) => (GDT_VALID as isize, SystemTime::from_date(d)),
                None => (
                    GDT_NONE as isize,
                    SystemTime {
                        year: 0,
                        month: 0,
                        weekday: 0,
                        day: 0,
                        hour: 0,
                        minute: 0,
                        second: 0,
                        millisecond: 0,
                    },
                ),
            };
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                SendMessageW(
                    self.inner.borrow().hwnd,
                    DTM_SETSYSTEMTIME,
                    flag as usize,
                    &st as *const _ as isize,
                );
            }
        }
        #[cfg(target_os = "macos")]
        {
            if let Some(d) = value {
                crate::platform::appkit::date_picker_set_value(
                    self.inner.borrow().stub_handle,
                    d.year,
                    d.month,
                    d.day,
                );
            }
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            let _ = value;
        }
    }

    /// Register a callback that fires when the user picks a different
    /// date (`DTN_DATETIMECHANGE`). The callback receives the new
    /// value: `Some(date)` if the user picked a valid date, or
    /// `None` if the control was cleared (only possible if the
    /// control was created with `allow_none` / `DTS_SHOWNONE`).
    ///
    /// # Implementation
    ///
    /// The `DTN_DATETIMECHANGE` notification carries a
    /// `NMDATETIMECHANGE` body whose `st` field is the new
    /// `SYSTEMTIME` value. This method registers a handler on
    /// the parent [`Frame`] via the internal `dtn_handlers`
    /// map (a `Box<dyn FnMut(isize)>` map that receives the
    /// full `lparam` so it can read the `NMDATETIMECHANGE`
    /// payload — the simpler code-only `notify_handlers` map
    /// that powers [`crate::Tab::on_selection_change`] does
    /// not have access to the `lparam`, which is why
    /// `on_date_change` cannot use it).
    ///
    /// # Cross-platform
    ///
    /// The callback is wired on every platform; on non-Windows
    /// hosts the `dtn_handlers` map is never invoked (there is
    /// no real `HWND`), so the callback simply never fires.
    /// This mirrors the cross-platform ergonomics of
    /// [`crate::Frame::set_drop_files_callback`].
    pub fn on_date_change<F: FnMut(Option<Date>) + 'static>(&self, frame: &Frame, mut callback: F) {
        let id = self.inner.borrow().id;
        #[cfg(target_os = "macos")]
        let handle = self.inner.borrow().stub_handle;
        #[cfg(target_os = "windows")]
        frame.register_dtn_handler(
            id,
            Box::new(move |lparam| {
                let nm_ptr = lparam as *const NmDateTimeChange;
                if nm_ptr.is_null() {
                    return;
                }
                let nm = unsafe { *nm_ptr };
                callback(nm.to_option());
            }),
        );
        #[cfg(target_os = "macos")]
        {
            frame.register_command_handler(
                id,
                Box::new(move || {
                    if let Some((y, m, d)) = crate::platform::appkit::date_picker_get_value(handle)
                    {
                        callback(Some(Date::new(y, m, d)));
                    } else {
                        callback(None);
                    }
                }),
            );
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            let _ = (frame, callback);
        }
    }

    /// Get the control ID.
    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    /// Get a `WidgetRef` for use with sizers.
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

// ── Widget trait ───────────────────────────────────────────────────────

impl Widget for DatePickerCtrlInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.hwnd as isize
        }
        #[cfg(target_os = "macos")]
        {
            self.stub_handle
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        0
    }

    fn set_position(&mut self, x: i32, y: i32) {
        self.rect.x = x;
        self.rect.y = y;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(
                self.hwnd,
                x,
                y,
                self.rect.width as i32,
                self.rect.height as i32,
                1,
            );
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::macos_widget_set_frame(
                self.parent_handle,
                self.stub_handle,
                self.rect,
            );
        }
    }

    fn set_size(&mut self, w: u32, h: u32) {
        self.rect.width = w;
        self.rect.height = h;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(self.hwnd, self.rect.x, self.rect.y, w as i32, h as i32, 1);
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::macos_widget_set_frame(
                self.parent_handle,
                self.stub_handle,
                self.rect,
            );
        }
    }

    fn rect(&self) -> Rect {
        self.rect
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            ShowWindow(self.hwnd, if visible { SW_SHOW } else { SW_HIDE });
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::macos_widget_set_visible(self.stub_handle, visible);
        }
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            EnableWindow(self.hwnd, if enabled { 1 } else { 0 });
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::macos_widget_set_enabled(self.stub_handle, enabled);
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────
//
// The value-extraction path (NMDATETIMECHANGE → Date) is the
// hot fix landed in v0.5.7. The tests below pin the
// constants and the round-trip conversion so that future
// refactors of the on_date_change handler cannot silently
// regress the value the user receives.

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    /// `Date::new` must store the supplied year/month/day
    /// verbatim. The constructor does not validate the ranges
    /// (caller's responsibility per the doc-comment), so a
    /// round-trip must be lossless.
    #[test]
    fn date_new_constructs_value() {
        let d = Date::new(2026, 6, 5);
        assert_eq!(d.year, 2026);
        assert_eq!(d.month, 6);
        assert_eq!(d.day, 5);
    }

    /// `Date` must be `Copy` and `Eq` — the on_date_change
    /// callback is `FnMut(Option<Date>)` and the user is
    /// expected to be able to compare the delivered date
    /// against model state with `==`.
    #[test]
    fn date_is_copy_and_eq() {
        let a = Date::new(2026, 6, 5);
        let b = a; // implicit copy
        assert_eq!(a, b);
        let c = Date::new(2026, 6, 6);
        assert_ne!(a, c);
    }

    /// `DTN_DATETIMECHANGE` must be the Win32
    /// `DTN_DATETIMECHANGE` value (0xFFFFFD09) so the
    /// frame's WM_NOTIFY arm matches what the
    /// `SysDateTimePick32` control actually emits. A
    /// regression here would silently break the dispatch
    /// (the arm would never fire).
    #[cfg(target_os = "windows")]
    #[test]
    fn dtn_datetimechange_constant_value() {
        assert_eq!(DTN_DATETIMECHANGE, 0xFFFFFD09_u32);
    }

    /// `NmDateTimeChange::to_option` must return
    /// `Some(date)` when `dw_flags` is `GDT_VALID` (0).
    /// This is the "happy path" — the user picked a
    /// real date.
    #[cfg(target_os = "windows")]
    #[test]
    fn nm_date_time_change_to_option_valid() {
        let st = SystemTime {
            year: 2026,
            month: 6,
            weekday: 5,
            day: 5,
            hour: 0,
            minute: 0,
            second: 0,
            millisecond: 0,
        };
        let nm = NmDateTimeChange {
            nmhdr: NMHDR {
                hwndFrom: std::ptr::null_mut(),
                idFrom: 0,
                code: 0,
            },
            dw_flags: GDT_VALID as u32,
            st,
        };
        assert_eq!(nm.to_option(), Some(Date::new(2026, 6, 5)));
    }

    /// `NmDateTimeChange::to_option` must return `None`
    /// when `dw_flags` is `GDT_NONE` (1). This is the
    /// "no date selected" case, which is only reachable
    /// when the control was created with
    /// `DTS_SHOWNONE` / `allow_none` set.
    #[cfg(target_os = "windows")]
    #[test]
    fn nm_date_time_change_to_option_none() {
        let nm = NmDateTimeChange {
            nmhdr: NMHDR {
                hwndFrom: std::ptr::null_mut(),
                idFrom: 0,
                code: 0,
            },
            dw_flags: GDT_NONE as u32,
            st: SystemTime {
                year: 0,
                month: 0,
                weekday: 0,
                day: 0,
                hour: 0,
                minute: 0,
                second: 0,
                millisecond: 0,
            },
        };
        assert_eq!(nm.to_option(), None);
    }

    /// `SystemTime::from_date` → `to_date` must be a
    /// lossless round-trip for the year/month/day fields.
    /// The time and weekday fields are zeroed by
    /// `from_date` (the control does not own a time of
    /// day in the date-only formats) and are not part of
    /// the `Date` payload.
    #[cfg(target_os = "windows")]
    #[test]
    fn systemtime_date_round_trip() {
        let d = Date::new(2026, 6, 5);
        let st = SystemTime::from_date(d);
        let d2 = st.to_date();
        assert_eq!(d, d2);
    }
}
