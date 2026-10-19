//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! wxTimePickerCtrl — a time-of-day picker (hours:minutes:seconds).
//!
//! On Windows this wraps the same `SysDateTimePick32` common control used
//! by [`DatePickerCtrl`](crate::DatePickerCtrl), created with the `DTS_TIMEFORMAT` style and an
//! explicit `DTM_SETFORMAT` string so the display always matches the
//! [`Time`] payload regardless of the locale. The Win32 constants and the
//! ABI structs (`SYSTEMTIME` / `NMDATETIMECHANGE`) are reused from
//! `date_picker_ctrl` — single definition in the middle.
//!
//! Use [`TimePickerCtrl::on_time_change`] to register a callback that
//! fires when the user picks a different time.

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
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[cfg(target_os = "windows")]
use crate::controls::date_picker_ctrl::{
    NmDateTimeChange, SystemTime, DTM_GETSYSTEMTIME, DTM_SETSYSTEMTIME, DTS_TIMEFORMAT, DTS_UPDOWN,
    GDT_NONE, GDT_VALID,
};

// DTM_SETFORMAT — set a custom display format on the SysDateTimePick32
// control (overrides the DTS_* style default). Format strings follow the
// `GetTimeFormat` pattern ("HH:mm:ss"). Not defined in
// `date_picker_ctrl.rs` (the date picker keeps the locale default).
#[cfg(target_os = "windows")]
const DTM_SETFORMAT: u32 = 0x1005;

// ── Public time type ──────────────────────────────────────────────────

/// A simple time of day used by [`TimePickerCtrl`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Time {
    pub hour: u32,   // 0..=23
    pub minute: u32, // 0..=59
    pub second: u32, // 0..=59
}

impl Time {
    /// Create a new time. The caller is responsible for ensuring that
    /// `hour` is in 0..=23 and `minute`/`second` in 0..=59.
    pub fn new(hour: u32, minute: u32, second: u32) -> Self {
        Self {
            hour,
            minute,
            second,
        }
    }
}

// Conversioni h/m/s verso/dal `SYSTEMTIME` Win32 (il campo data viene
// impostato a una data valida perché il controllo accetti il set anche in
// formato tempo; il display in `DTS_TIMEFORMAT` lo ignora).
#[cfg(target_os = "windows")]
impl SystemTime {
    fn from_time(t: Time) -> Self {
        SystemTime {
            year: 1970,
            month: 1,
            weekday: 4, // giovedì
            day: 1,
            hour: t.hour as u16,
            minute: t.minute as u16,
            second: t.second as u16,
            millisecond: 0,
        }
    }

    fn to_time(self) -> Time {
        Time {
            hour: self.hour as u32,
            minute: self.minute as u32,
            second: self.second as u32,
        }
    }
}

// ── Inner type ─────────────────────────────────────────────────────────

struct TimePickerInner {
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
pub struct TimePickerCtrl {
    inner: Rc<RefCell<TimePickerInner>>,
}

impl TimePickerCtrl {
    /// Create a new time-picker control as a child of `parent`, showing
    /// hours, minutes and seconds (`HH:mm:ss`).
    pub fn new<W: Window>(parent_in: &W) -> Self {
        Self::create(parent_in, false)
    }

    /// Create a time-picker that only shows hours and minutes (`HH:mm`).
    /// `get_value` still returns the seconds field (usually 0).
    pub fn new_hm<W: Window>(parent_in: &W) -> Self {
        Self::create(parent_in, true)
    }

    fn create<W: Window>(parent_in: &W, hour_minute_only: bool) -> Self {
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
                WS_CHILD | WS_VISIBLE | DTS_TIMEFORMAT | DTS_UPDOWN,
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

        // Formato esplicito: l'API espone sempre h/m/s, quindi il display
        // non deve dipendere dalla locale (es. "h:mm tt" in en-US).
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / format string).
        unsafe {
            let format = if hour_minute_only {
                "HH:mm"
            } else {
                "HH:mm:ss"
            };
            // La classe SysDateTimePick32 può essere registrata come ANSI:
            // in quel caso il controllo legge il formato come stringa
            // stretta e la variante UTF-16 si ferma al primo NUL (da qui
            // un display con la sola ora). Si invia prima la versione
            // wide e, se il testo del controllo non contiene i separatori,
            // si riprova con la stringa a byte.
            let read_text = |h: HWND| {
                let mut buf = [0u16; 64];
                let n = GetWindowTextW(h, buf.as_mut_ptr(), buf.len() as i32);
                String::from_utf16_lossy(&buf[..n as usize])
            };
            let wide_format = to_wide(format);
            SendMessageW(hwnd, DTM_SETFORMAT, 0, wide_format.as_ptr() as isize);
            let text_w = read_text(hwnd);
            if !text_w.contains(':') {
                // Classe ANSI confermata: si riprova con la stringa a byte.
                let mut bytes = format.as_bytes().to_vec();
                bytes.push(0);
                SendMessageA(hwnd, DTM_SETFORMAT, 0, bytes.as_ptr() as isize);
            }
        }

        #[cfg(target_os = "macos")]
        let parent_handle = crate::platform::window_handle(parent_in);
        #[cfg(target_os = "macos")]
        let stub_handle = crate::platform::appkit::create_time_picker(
            parent_handle,
            id,
            Rect::new(0, 0, 160, 24),
            hour_minute_only,
        );

        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = (parent_in, hour_minute_only);

        TimePickerCtrl {
            inner: Rc::new(RefCell::new(TimePickerInner {
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

    /// Return the current time, or `None` if the control reports no
    /// value (should not happen for a time picker without a "none"
    /// option).
    pub fn get_value(&self) -> Option<Time> {
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
            // SAFETY: Win32 FFI call with validated arguments (HWND / buffer large enough for the output).
            let result = unsafe {
                SendMessageW(
                    self.inner.borrow().hwnd,
                    DTM_GETSYSTEMTIME,
                    0,
                    &mut st as *mut _ as isize,
                )
            };
            if result as u16 == GDT_VALID {
                Some(st.to_time())
            } else {
                None
            }
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::time_picker_get_value(self.inner.borrow().stub_handle)
                .map(|(h, m, s)| Time::new(h, m, s))
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            None
        }
    }

    /// Set the current time. `None` clears the value (Windows:
    /// `GDT_NONE`; a no-op on hosts without a "none" state).
    pub fn set_value(&self, value: Option<Time>) {
        #[cfg(target_os = "windows")]
        {
            let (flag, st) = match value {
                Some(t) => (GDT_VALID as isize, SystemTime::from_time(t)),
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
            // SAFETY: Win32 FFI call with validated arguments (HWND / SYSTEMTIME buffer).
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
            if let Some(t) = value {
                crate::platform::appkit::time_picker_set_value(
                    self.inner.borrow().stub_handle,
                    t.hour,
                    t.minute,
                    t.second,
                );
            }
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            let _ = value;
        }
    }

    /// Register a callback that fires when the user picks a different
    /// time. The same `DTN_DATETIMECHANGE` notification used by
    /// [`DatePickerCtrl::on_date_change`](crate::DatePickerCtrl::on_date_change) is delivered on Windows
    /// (routed through the frame's `dtn_handlers` map); on macOS the
    /// picker's action message is routed through `register_command_handler`
    /// and the value is re-read from the control.
    pub fn on_time_change<F: FnMut(Option<Time>) + 'static>(&self, frame: &Frame, mut callback: F) {
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
                if nm.dw_flags as u16 == GDT_VALID {
                    callback(Some(nm.st.to_time()));
                } else {
                    callback(None);
                }
            }),
        );
        #[cfg(target_os = "macos")]
        {
            frame.register_command_handler(
                id,
                Box::new(move || {
                    if let Some((h, m, s)) = crate::platform::appkit::time_picker_get_value(handle)
                    {
                        callback(Some(Time::new(h, m, s)));
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

impl Widget for TimePickerInner {
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
        // SAFETY: Win32 FFI call with validated arguments (HWND) and a buffer large enough for the output.
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
        // SAFETY: Win32 FFI call with validated arguments (HWND) and a buffer large enough for the output.
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
        // SAFETY: Win32 FFI call with validated arguments (HWND).
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
        // SAFETY: Win32 FFI call with validated arguments (HWND).
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `Time::new` must store the supplied hour/minute/second verbatim
    /// (ranges are the caller's responsibility per the doc-comment).
    #[test]
    fn time_new_constructs_value() {
        let t = Time::new(14, 30, 5);
        assert_eq!(t.hour, 14);
        assert_eq!(t.minute, 30);
        assert_eq!(t.second, 5);
    }

    /// `Time` must be `Copy` and `Eq` — the on_time_change callback is
    /// `FnMut(Option<Time>)` and users compare values with `==`.
    #[test]
    fn time_is_copy_and_eq() {
        let a = Time::new(14, 30, 5);
        let b = a; // implicit copy
        assert_eq!(a, b);
        let c = Time::new(14, 30, 6);
        assert_ne!(a, c);
    }

    /// `SystemTime::from_time` → `to_time` must be a lossless round-trip
    /// for the h/m/s fields (the date fields are only a placeholder so
    /// the Win32 control accepts the set in `DTS_TIMEFORMAT`).
    #[cfg(target_os = "windows")]
    #[test]
    fn systemtime_time_round_trip() {
        let t = Time::new(23, 59, 58);
        let st = SystemTime::from_time(t);
        assert_eq!(st.to_time(), t);
    }

    /// The Windows path reuses `DTN_DATETIMECHANGE` from
    /// `date_picker_ctrl` — the frame's WM_NOTIFY arm matches this exact
    /// value to route the notification, so pin it here too.
    #[cfg(target_os = "windows")]
    #[test]
    fn dtn_datetimechange_reused_constant() {
        assert_eq!(
            crate::controls::date_picker_ctrl::DTN_DATETIMECHANGE,
            0xFFFFFD09_u32
        );
    }
}
