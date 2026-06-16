//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Progress bar (`wxGauge`).
//!
//! On Windows, the progress bar is realised with the common control
//! class `msctls_progress32`. The control is created in one of three
//! modes:
//!
//! - **Determinate** (default): the bar fills from 0 to a maximum
//!   value as the caller pushes new values into it. The bar can be
//!   smooth or segmented (`PBS_SMOOTH`).
//! - **Indeterminate** (`PBS_MARQUEE`): the bar continuously
//!   scrolls; the caller does not push values. Indeterminate bars
//!   ignore `set_value` / `set_range`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::geometry::{Colour, Rect};
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

// ── Win32 progress bar constants ────────────────────────────────────

/// `PBM_SETRANGE` — set the min/max range.
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const PBM_SETRANGE: u32 = 0x0401;
/// `PBM_SETPOS` — set the current position.
#[cfg(target_os = "windows")]
const PBM_SETPOS: u32 = 0x0402;
/// `PBM_DELTAPOS` — increment the position by a delta.
#[cfg(target_os = "windows")]
const PBM_DELTAPOS: u32 = 0x0403;
/// `PBM_SETSTEP` — set the step increment used by `PBM_STEPIT`.
#[cfg(target_os = "windows")]
const PBM_SETSTEP: u32 = 0x0404;
/// `PBM_STEPIT` — advance the position by the step.
#[cfg(target_os = "windows")]
const PBM_STEPIT: u32 = 0x0405;
/// `PBM_SETRANGE32` — set 32-bit min/max range.
#[cfg(target_os = "windows")]
const PBM_SETRANGE32: u32 = 0x0406;
/// `PBM_GETPOS` — get the current position.
#[cfg(target_os = "windows")]
const PBM_GETPOS: u32 = 0x0408;
/// `PBM_SETBARCOLOR` — set the bar colour.
#[cfg(target_os = "windows")]
const PBM_SETBARCOLOR: u32 = 0x0409;
/// `PBM_SETMARQUEE` — start/stop the marquee animation.
#[cfg(target_os = "windows")]
const PBM_SETMARQUEE: u32 = 0x040A;
/// `PBS_SMOOTH` — draw the bar as a single smooth rectangle (no
/// segments).
#[cfg(target_os = "windows")]
const PBS_SMOOTH: u32 = 0x01;
/// `PBS_VERTICAL` — vertical orientation.
#[cfg(target_os = "windows")]
const PBS_VERTICAL: u32 = 0x04;
/// `PBS_MARQUEE` — indeterminate / marquee mode.
#[cfg(target_os = "windows")]
const PBS_MARQUEE: u32 = 0x08;

// ── Style enum ───────────────────────────────────────────────────────

/// Visual style of a `Gauge`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GaugeStyle {
    /// Default horizontal segmented bar.
    Horizontal,
    /// Single smooth horizontal rectangle (no segments).
    SmoothHorizontal,
    /// Vertical segmented bar.
    Vertical,
    /// Single smooth vertical rectangle.
    SmoothVertical,
}

impl GaugeStyle {
    #[cfg(target_os = "windows")]
    fn to_native(self) -> u32 {
        match self {
            GaugeStyle::Horizontal => 0,
            GaugeStyle::SmoothHorizontal => PBS_SMOOTH,
            GaugeStyle::Vertical => PBS_VERTICAL,
            GaugeStyle::SmoothVertical => PBS_VERTICAL | PBS_SMOOTH,
        }
    }
}

// ── Inner type ───────────────────────────────────────────────────────

struct GaugeInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    rect: Rect,
    range: i32,
    value: i32,
    indeterminate: bool,
    enabled: bool,
    visible: bool,
}

#[derive(Clone)]
pub struct Gauge {
    inner: Rc<RefCell<GaugeInner>>,
}

impl Gauge {
    /// Create a new horizontal progress bar with the given range.
    pub fn new<W: Window>(parent_in: &W, range: i32) -> Self {
        Self::new_with_style(parent_in, range, GaugeStyle::Horizontal, false)
    }

    /// Create a new smooth (non-segmented) horizontal progress bar.
    pub fn new_smooth<W: Window>(parent_in: &W, range: i32) -> Self {
        Self::new_with_style(parent_in, range, GaugeStyle::SmoothHorizontal, false)
    }

    /// Create a new vertical progress bar.
    pub fn new_vertical<W: Window>(parent_in: &W, range: i32) -> Self {
        Self::new_with_style(parent_in, range, GaugeStyle::Vertical, false)
    }

    /// Create a new progress bar with the given style.
    pub fn new_with_style<W: Window>(
        parent_in: &W,
        range: i32,
        style: GaugeStyle,
        indeterminate: bool,
    ) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = parent_in.hwnd();
            let wide_class = to_wide("msctls_progress32");
            let mut win_style = WS_CHILD | WS_VISIBLE | style.to_native();
            if indeterminate {
                win_style |= PBS_MARQUEE;
            }
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                win_style,
                0,
                0,
                200,
                20,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = (parent_in, style, indeterminate);

        let g = Gauge {
            inner: Rc::new(RefCell::new(GaugeInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                rect: Rect::new(0, 0, 200, 20),
                range,
                value: 0,
                indeterminate,
                enabled: true,
                visible: true,
            })),
        };

        // Push the initial range to the control.
        g.set_range(range);

        // If we're in marquee mode, start the animation.
        if indeterminate {
            g.pulse();
        }

        g
    }

    /// Set the bar's range (max value). The current value is preserved
    /// (clamped to the new range).
    pub fn set_range(&self, range: i32) {
        {
            let mut inner = self.inner.borrow_mut();
            inner.range = range;
            if inner.value > range {
                inner.value = range;
            }
        }
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, PBM_SETRANGE32, 0, range as isize);
        }
    }

    /// Return the current range (max value).
    pub fn get_range(&self) -> i32 {
        self.inner.borrow().range
    }

    /// Set the current value (clamped to `[0, range]`).
    pub fn set_value(&self, value: i32) {
        let clamped = {
            let mut inner = self.inner.borrow_mut();
            if inner.indeterminate {
                // Indeterminate bars ignore position; do nothing.
                return;
            }
            let v = value.max(0).min(inner.range);
            inner.value = v;
            v
        };
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, PBM_SETPOS, clamped as usize, 0);
        }
    }

    /// Return the current value.
    pub fn get_value(&self) -> i32 {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: FFI call to SendMessageW; `hwnd` is a live window and `msg` / `wParam` / `lParam` are valid for that window.
            let v = unsafe { SendMessageW(self.inner.borrow().hwnd, PBM_GETPOS, 0, 0) };
            self.inner.borrow_mut().value = v as i32;
            v as i32
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.inner.borrow().value
        }
    }

    /// Increment the position by `delta` (clamped to the range).
    /// Returns the new value.
    pub fn increment(&self, delta: i32) -> i32 {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.indeterminate {
                return 0;
            }
            let new_value = (inner.value + delta).max(0).min(inner.range);
            inner.value = new_value;
        }
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, PBM_DELTAPOS, delta as usize, 0);
        }
        self.inner.borrow().value
    }

    /// Set the step size used by [`Gauge::step`].
    pub fn set_step(&self, step: i32) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, PBM_SETSTEP, step as usize, 0);
        }
    }

    /// Advance the position by the configured step size.
    pub fn step(&self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, PBM_STEPIT, 0, 0);
        }
    }

    /// Start / restart the marquee animation (for indeterminate
    /// gauges).
    pub fn pulse(&self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            // wParam = 1 to start, 0 to stop; lParam = update time in ms.
            SendMessageW(
                self.inner.borrow().hwnd,
                PBM_SETMARQUEE,
                1,
                30, // ~33 fps
            );
        }
    }

    /// Stop the marquee animation.
    pub fn stop_pulse(&self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, PBM_SETMARQUEE, 0, 0);
        }
    }

    /// Set the colour of the filled portion of the bar.
    pub fn set_bar_colour(&self, colour: Colour) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(
                self.inner.borrow().hwnd,
                PBM_SETBARCOLOR,
                0,
                colour.to_colorref() as isize,
            );
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

impl Widget for GaugeInner {
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
