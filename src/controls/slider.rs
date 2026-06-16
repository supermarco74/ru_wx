//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Continuous-value slider (`wxSlider`).
//!
//! On Windows, the slider is realised with the common control class
//! `msctls_trackbar32`. The control exposes a value in `[min, max]`
//! with a movable thumb. Two orientations are supported (`Horizontal`
//! and `Vertical`); ticks can be displayed or hidden.
//!
//! Changes are reported as `WM_HSCROLL` / `WM_VSCROLL` to the parent
//! window (with `lParam` set to the trackbar's `HWND`). The frame's
//! WndProc dispatches those via [`Frame::register_scroll_handler`].

use std::cell::RefCell;
use std::rc::Rc;

use crate::window::frame::Frame;
use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};

use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── Win32 trackbar constants ─────────────────────────────────────────

// Values from `commctrl.h`: TBM_GETPOS is (WM_USER), TBM_SETPOS is
// (WM_USER+5), etc. Pinned in `tbm_constants_match_commctrl_h` below.
#[cfg(target_os = "windows")]
const WM_USER: u32 = 0x0400;
#[cfg(target_os = "windows")]
const TBM_GETPOS: u32 = WM_USER; // (WM_USER)
#[cfg(target_os = "windows")]
const TBM_SETPOS: u32 = WM_USER + 5;
#[cfg(target_os = "windows")]
const TBM_GETRANGEMIN: u32 = WM_USER + 1;
#[cfg(target_os = "windows")]
const TBM_GETRANGEMAX: u32 = WM_USER + 2;
#[cfg(target_os = "windows")]
const TBM_SETRANGEMIN: u32 = WM_USER + 7; // fRedraw = wparam
#[cfg(target_os = "windows")]
const TBM_SETRANGEMAX: u32 = WM_USER + 8;
#[cfg(target_os = "windows")]
const TBM_SETTICFREQ: u32 = WM_USER + 20;
#[cfg(target_os = "windows")]
const TBM_SETLINESIZE: u32 = WM_USER + 23;
#[cfg(target_os = "windows")]
const TBM_SETPAGESIZE: u32 = WM_USER + 21;

#[cfg(target_os = "windows")]
const TBS_HORZ: u32 = 0x0000; // default
#[cfg(target_os = "windows")]
const TBS_VERT: u32 = 0x0002;
#[cfg(target_os = "windows")]
const TBS_AUTOTICKS: u32 = 0x0001;
#[cfg(target_os = "windows")]
const TBS_BOTH: u32 = 0x0008; // ticks on both sides
#[cfg(target_os = "windows")]
#[allow(dead_code)] // Win32 ABI surface — kept for completeness of the TBS_ style table
const TBS_TOP: u32 = 0x0004;
#[cfg(target_os = "windows")]
const TBS_LEFT: u32 = 0x0004;
#[cfg(target_os = "windows")]
#[allow(dead_code)] // Win32 ABI surface — kept for completeness of the TBS_ style table
const TBS_NOTICKS: u32 = 0x0010;

// ── Inner type ───────────────────────────────────────────────────────

struct SliderInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(target_os = "windows")]
    scroll_frame: Option<Frame>,
    id: u16,
    rect: Rect,
    min: i32,
    max: i32,
    value: i32,
    enabled: bool,
    visible: bool,
}

#[derive(Clone)]
pub struct Slider {
    inner: Rc<RefCell<SliderInner>>,
}

impl Slider {
    /// Create a new horizontal slider with the given value range.
    pub fn new<W: Window>(parent_in: &W, min: i32, max: i32, initial: i32) -> Self {
        Self::new_internal(parent_in, min, max, initial, false)
    }

    /// Create a new vertical slider with the given value range.
    pub fn new_vertical<W: Window>(parent_in: &W, min: i32, max: i32, initial: i32) -> Self {
        Self::new_internal(parent_in, min, max, initial, true)
    }

    fn new_internal<W: Window>(
        parent_in: &W,
        min: i32,
        max: i32,
        initial: i32,
        vertical: bool,
    ) -> Self {
        let id = next_control_id();
        let initial = initial.max(min).min(max);

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = parent_in.hwnd();
            let wide_class = to_wide("msctls_trackbar32");
            let mut style = WS_CHILD | WS_VISIBLE | TBS_AUTOTICKS;
            if vertical {
                style |= TBS_VERT | TBS_LEFT;
            } else {
                style |= TBS_HORZ | TBS_BOTH;
            }
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                style,
                0,
                0,
                200,
                if vertical { 200 } else { 30 },
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = (parent_in, vertical);

        let s = Slider {
            inner: Rc::new(RefCell::new(SliderInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(target_os = "windows")]
                scroll_frame: None,
                id,
                rect: Rect::new(0, 0, 200, if vertical { 200 } else { 30 }),
                min,
                max,
                value: initial,
                enabled: true,
                visible: true,
            })),
        };

        s.set_range(min, max);
        s.set_value(initial);
        s
    }

    /// Set the slider's range.
    pub fn set_range(&self, min: i32, max: i32) {
        {
            let mut inner = self.inner.borrow_mut();
            inner.min = min;
            inner.max = max;
            if inner.value < min {
                inner.value = min;
            } else if inner.value > max {
                inner.value = max;
            }
        }
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            // SETRANGEMIN fRedraw = 1, SETRANGEMAX fRedraw = 1
            SendMessageW(self.inner.borrow().hwnd, TBM_SETRANGEMIN, 1, min as isize);
            SendMessageW(self.inner.borrow().hwnd, TBM_SETRANGEMAX, 1, max as isize);
        }
    }

    /// Set the current value.
    pub fn set_value(&self, value: i32) {
        let v = {
            let mut inner = self.inner.borrow_mut();
            let v = value.max(inner.min).min(inner.max);
            inner.value = v;
            v
        };
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, TBM_SETPOS, 1, v as isize);
        }
    }

    /// Return the current value.
    pub fn get_value(&self) -> i32 {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: FFI call to SendMessageW; `hwnd` is a live window and `msg` / `wParam` / `lParam` are valid for that window.
            let v = unsafe { SendMessageW(self.inner.borrow().hwnd, TBM_GETPOS, 0, 0) };
            self.inner.borrow_mut().value = v as i32;
            v as i32
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.inner.borrow().value
        }
    }

    /// Return the current range minimum.
    pub fn get_min(&self) -> i32 {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: FFI call to SendMessageW; `hwnd` is a live window and `msg` / `wParam` / `lParam` are valid for that window.
            unsafe { SendMessageW(self.inner.borrow().hwnd, TBM_GETRANGEMIN, 0, 0) as i32 }
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.inner.borrow().min
        }
    }

    /// Return the current range maximum.
    pub fn get_max(&self) -> i32 {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: FFI call to SendMessageW; `hwnd` is a live window and `msg` / `wParam` / `lParam` are valid for that window.
            unsafe { SendMessageW(self.inner.borrow().hwnd, TBM_GETRANGEMAX, 0, 0) as i32 }
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.inner.borrow().max
        }
    }

    /// Return the current range as `(min, max)`.
    pub fn get_range(&self) -> (i32, i32) {
        (self.get_min(), self.get_max())
    }

    /// Set the frequency of intermediate tick marks (e.g. every `freq`
    /// units).
    pub fn set_tick_freq(&self, freq: i32) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, TBM_SETTICFREQ, freq as usize, 0);
        }
    }

    /// Set the line / page step sizes (the small and large scroll
    /// deltas used by arrow keys / Page-Up / Page-Down).
    pub fn set_page_size(&self, page: i32) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, TBM_SETPAGESIZE, 0, page as isize);
        }
    }

    /// Set the line step (arrow-key increment).
    pub fn set_line_size(&self, line: i32) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, TBM_SETLINESIZE, 0, line as isize);
        }
    }

    /// Register a callback fired when the slider value changes
    /// (thumb drag, arrow keys, page up/down, or `set_value`).
    pub fn on_value_change<F: FnMut() + 'static>(&self, frame: &Frame, mut callback: F) {
        let inner = self.inner.clone();
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            self.inner.borrow_mut().scroll_frame = Some(frame.clone());
            let wrapper = move |_code: u16, _pos: i32| {
                let hwnd = inner.borrow().hwnd;
                // SAFETY: FFI call to SendMessageW; `hwnd` is a live trackbar.
                let v = unsafe { SendMessageW(hwnd, TBM_GETPOS, 0, 0) } as i32;
                inner.borrow_mut().value = v;
                callback();
            };
            frame.register_scroll_handler(hwnd as isize, wrapper);
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (frame, callback);
        }
    }

    /// The control's id.
    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    /// Get a `WidgetRef` for use with sizers.
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

#[cfg(target_os = "windows")]
impl Drop for Slider {
    fn drop(&mut self) {
        if Rc::strong_count(&self.inner) == 1 {
            let inner = self.inner.borrow();
            if let Some(ref frame) = inner.scroll_frame {
                frame.unregister_scroll_handler(inner.hwnd as isize);
            }
        }
    }
}

impl Widget for SliderInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.hwnd as isize
        }
        #[cfg(not(target_os = "windows"))]
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
    }

    fn set_size(&mut self, w: u32, h: u32) {
        self.rect.width = w;
        self.rect.height = h;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(self.hwnd, self.rect.x, self.rect.y, w as i32, h as i32, 1);
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
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    /// Pin the trackbar message ids to the values in `commctrl.h` so
    /// a future edit cannot silently reintroduce wrong offsets
    /// (e.g. `TBM_GETPOS` is `WM_USER + 0`, *not* `WM_USER + 21`).
    #[test]
    fn tbm_constants_match_commctrl_h() {
        assert_eq!(TBM_GETPOS, 0x0400);
        assert_eq!(TBM_GETRANGEMIN, 0x0401);
        assert_eq!(TBM_GETRANGEMAX, 0x0402);
        assert_eq!(TBM_SETPOS, 0x0405);
        assert_eq!(TBM_SETRANGEMIN, 0x0407);
        assert_eq!(TBM_SETRANGEMAX, 0x0408);
        assert_eq!(TBM_SETTICFREQ, 0x0414);
        assert_eq!(TBM_SETPAGESIZE, 0x0415);
        assert_eq!(TBM_SETLINESIZE, 0x0417);
    }
}
