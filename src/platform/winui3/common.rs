//! Common operations available on every widget.
//!
//! Rather than duplicating `enable`/`show`/`set_position`/... on every
//! struct in `app.rs` / `widgets.rs`, we expose them once through a
//! trait that operates on the widget's `HWND`. Every concrete widget
//! gets an `impl WidgetCommon for X { fn hwnd(&self) -> HWND { self.hwnd } }`
//! blanket implementation; the user code then writes:
//!
//! ```ignore
//! use ru_wx::platform::winui3::common::WidgetCommon;
//! button.enable(false);
//! if panel.is_visible() { /* ... */ }
//! ```
//!
//! All methods are safe to call on a null handle — they early-return
//! in that case.

#![allow(non_snake_case)]

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, IsWindowEnabled};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetDlgCtrlID, GetWindowRect, IsWindowVisible, SetWindowPos, ShowWindow, SWP_NOZORDER, SW_HIDE,
    SW_SHOW,
};

/// Trait that exposes universal operations on any widget that carries an HWND.
pub trait WidgetCommon {
    /// Returns the native window handle. Implementations should always
    /// return the most up-to-date HWND stored on the widget.
    fn hwnd_handle(&self) -> HWND;

    /// Enable (`true`) or disable (`false`) the widget for user input.
    /// Returns `true` if the window was previously disabled.
    fn enable(&self, enabled: bool) -> bool {
        let h = self.hwnd_handle();
        if h.is_null() {
            return false;
        }
        unsafe { EnableWindow(h, if enabled { 1 } else { 0 }) == 0 }
    }

    /// Returns `true` if the widget accepts user input.
    fn is_enabled(&self) -> bool {
        let h = self.hwnd_handle();
        if h.is_null() {
            return false;
        }
        unsafe { IsWindowEnabled(h) != 0 }
    }

    /// Make the widget visible.
    fn show(&self) {
        let h = self.hwnd_handle();
        if h.is_null() {
            return;
        }
        unsafe {
            ShowWindow(h, SW_SHOW);
        }
    }

    /// Hide the widget.
    fn hide(&self) {
        let h = self.hwnd_handle();
        if h.is_null() {
            return;
        }
        unsafe {
            ShowWindow(h, SW_HIDE);
        }
    }

    /// Returns `true` if the widget is currently visible.
    fn is_visible(&self) -> bool {
        let h = self.hwnd_handle();
        if h.is_null() {
            return false;
        }
        unsafe { IsWindowVisible(h) != 0 }
    }

    /// Place the widget at `(x, y)` relative to its parent's client
    /// area, leaving its current size untouched.
    fn set_position(&self, x: i32, y: i32) {
        let h = self.hwnd_handle();
        if h.is_null() {
            return;
        }
        unsafe {
            SetWindowPos(
                h,
                std::ptr::null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOZORDER | 0x0001, /* SWP_NOSIZE */
            );
        }
    }

    /// Resize the widget to `(w, h)`. Position is left untouched.
    fn set_size(&self, w: i32, hgt: i32) {
        let h = self.hwnd_handle();
        if h.is_null() {
            return;
        }
        unsafe {
            SetWindowPos(
                h,
                std::ptr::null_mut(),
                0,
                0,
                w,
                hgt,
                SWP_NOZORDER | 0x0002, /* SWP_NOMOVE */
            );
        }
    }

    /// Move and resize in one shot.
    fn set_rect(&self, x: i32, y: i32, w: i32, h: i32) {
        let hh = self.hwnd_handle();
        if hh.is_null() {
            return;
        }
        unsafe {
            SetWindowPos(hh, std::ptr::null_mut(), x, y, w, h, SWP_NOZORDER);
        }
    }

    /// Returns the absolute (`x`, `y`) position of the widget's window
    /// rectangle. Returns `(0, 0)` if the widget or its parent has no
    /// window.
    fn get_position(&self) -> (i32, i32) {
        let h = self.hwnd_handle();
        if h.is_null() {
            return (0, 0);
        }
        let mut rect: windows_sys::Win32::Foundation::RECT = unsafe { std::mem::zeroed() };
        unsafe {
            GetWindowRect(h, &mut rect);
        }
        (rect.left, rect.top)
    }

    /// Returns `(width, height)` of the widget's window rectangle.
    fn get_size(&self) -> (i32, i32) {
        let h = self.hwnd_handle();
        if h.is_null() {
            return (0, 0);
        }
        let mut rect: windows_sys::Win32::Foundation::RECT = unsafe { std::mem::zeroed() };
        unsafe {
            GetWindowRect(h, &mut rect);
        }
        (rect.right - rect.left, rect.bottom - rect.top)
    }

    /// Returns the Win32 control id (the `HMENU` argument that was
    /// passed to `CreateWindowExW`). Returns `0` if the widget has no
    /// assigned id.
    fn id(&self) -> u16 {
        let h = self.hwnd_handle();
        if h.is_null() {
            return 0;
        }
        unsafe { GetDlgCtrlID(h) as u16 }
    }
}

/// Free-function equivalents that work for any HWND, mainly so that
/// subclasses of `Window` (which only carry an HWND field) can use the
/// same utilities without implementing the trait.
#[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper; `h` must be live
pub fn enable_hwnd(h: HWND, enabled: bool) -> bool {
    if h.is_null() {
        return false;
    }
    unsafe { EnableWindow(h, if enabled { 1 } else { 0 }) == 0 }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper; `h` must be live
pub fn is_enabled_hwnd(h: HWND) -> bool {
    if h.is_null() {
        return false;
    }
    unsafe { IsWindowEnabled(h) != 0 }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)] // thin FFI wrapper; `h` must be live
pub fn set_rect_hwnd(h: HWND, x: i32, y: i32, w: i32, hh: i32) {
    if h.is_null() {
        return;
    }
    unsafe {
        SetWindowPos(h, std::ptr::null_mut(), x, y, w, hh, SWP_NOZORDER);
    }
}

// ===========================================================================
// Blanket `WidgetCommon` impls for every concrete widget type.
//
// Each type has an `hwnd()` method already in place; the blanket impl
// just forwards to it.
// ===========================================================================

impl WidgetCommon for super::app::Frame {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}
impl WidgetCommon for super::app::FrameBuilder {
    fn hwnd_handle(&self) -> HWND {
        std::ptr::null_mut()
    }
}
impl WidgetCommon for super::app::Button {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}
impl WidgetCommon for super::app::StaticText {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}
impl WidgetCommon for super::app::TextCtrl {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}
impl WidgetCommon for super::app::CheckBox {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}
impl WidgetCommon for super::app::RadioButton {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}
impl WidgetCommon for super::app::Panel {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}
impl WidgetCommon for super::app::Choice {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}
impl WidgetCommon for super::app::ListBox {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}
impl WidgetCommon for super::app::ListView {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}
impl WidgetCommon for super::app::ListBoxUWP {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}
impl WidgetCommon for super::app::Slider {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}
impl WidgetCommon for super::app::Gauge {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}
impl WidgetCommon for super::app::ComboBox {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}
impl WidgetCommon for super::app::RadioBox {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}
impl WidgetCommon for super::app::StatusBar {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}
impl WidgetCommon for super::app::Tab {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}

impl WidgetCommon for super::widgets::Hyperlink {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}
impl WidgetCommon for super::widgets::StaticLine {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}
impl WidgetCommon for super::widgets::StaticBox {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}
impl WidgetCommon for super::widgets::BitmapButton {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}
impl WidgetCommon for super::widgets::ProgressBar {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}
impl WidgetCommon for super::widgets::SpinCtrl {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}
impl WidgetCommon for super::widgets::Splitter {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}
impl WidgetCommon for super::widgets::SearchCtrl {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}

impl WidgetCommon for super::tree::TreeCtrl {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}

impl WidgetCommon for super::grid::Grid {
    fn hwnd_handle(&self) -> HWND {
        self.hwnd()
    }
}
