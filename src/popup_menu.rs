//! `wxPopupMenu` — a context-menu helper.
//!
//! `Menu` already supports being shown as a popup via
//! [`Menu::popup_at_cursor`]. This module provides a thin wrapper
//! type that emphasises the popup use case (vs. a normal menu bar
//! dropdown) and adds a couple of convenience methods.
//!
//! # Example
//! ```no_run
//! use ru_wx::popup_menu::PopupMenu;
//! use ru_wx::frame::Frame;
//!
//! let mut popup = PopupMenu::new();
//! let frame = Frame::builder().with_title("x").with_size(100, 100).build();
//! popup.append("Cut", &frame, || {});
//! // ... later, on right-click:
//! popup.popup(&frame);
//! ```

use crate::frame::Frame;
use crate::menu::Menu;

/// A `Menu` that is shown on-demand (typically on right-click) rather
/// than as a dropdown of a menu bar.
pub struct PopupMenu {
    inner: Menu,
}

impl PopupMenu {
    /// Create an empty popup menu.
    pub fn new() -> Self {
        PopupMenu {
            inner: Menu::new(""),
        }
    }

    /// Append a normal enabled item. See [`Menu::append`].
    pub fn append<F: FnMut() + 'static>(&mut self, label: &str, frame: &Frame, callback: F) {
        self.inner.append(label, frame, callback);
    }

    /// Append a disabled (greyed) item.
    pub fn append_disabled(&mut self, label: &str) {
        self.inner.append_disabled(label);
    }

    /// Append a checkable item. Returns the item id.
    pub fn append_check_item<F: FnMut() + 'static>(
        &mut self,
        label: &str,
        frame: &Frame,
        callback: F,
    ) -> u16 {
        self.inner.append_check_item(label, frame, callback)
    }

    /// Append a radio item. Returns the item id.
    pub fn append_radio_item<F: FnMut() + 'static>(
        &mut self,
        label: &str,
        frame: &Frame,
        callback: F,
    ) -> u16 {
        self.inner.append_radio_item(label, frame, callback)
    }

    /// Append a horizontal separator.
    pub fn append_separator(&mut self) {
        self.inner.append_separator();
    }

    /// Append an item with a coloured icon. See [`Menu::append_with_colour_icon`].
    pub fn append_with_colour_icon<F: FnMut() + 'static>(
        &mut self,
        label: &str,
        colour: crate::geometry::Colour,
        frame: &Frame,
        callback: F,
    ) {
        self.inner
            .append_with_colour_icon(label, colour, frame, callback);
    }

    /// Append an item with an SVG icon. See [`Menu::append_with_svg_icon`].
    pub fn append_with_svg_icon<F: FnMut() + 'static>(
        &mut self,
        label: &str,
        svg_bytes: &[u8],
        icon_size: u32,
        frame: &Frame,
        callback: F,
    ) {
        self.inner
            .append_with_svg_icon(label, svg_bytes, icon_size, frame, callback);
    }

    /// Set the checked state of a checkable item. See [`Menu::check_item`].
    pub fn check_item(&mut self, id: u16, check: bool) -> bool {
        self.inner.check_item(id, check)
    }

    /// Show the popup at the current cursor position. The supplied
    /// `frame` is used as the owner window (so it receives the
    /// resulting `WM_COMMAND` and dismisses the popup).
    #[cfg(target_os = "windows")]
    pub fn popup(&self, frame: &Frame) {
        self.inner.popup_at_cursor(frame.hwnd());
    }

    /// Non-Windows stub.
    #[cfg(not(target_os = "windows"))]
    pub fn popup(&self, _frame: &Frame) {}

    /// Show the popup at a specific (x, y) position in screen
    /// coordinates.
    #[cfg(target_os = "windows")]
    pub fn popup_at(&self, frame: &Frame, x: i32, y: i32) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            PostMessageW, SetForegroundWindow, TrackPopupMenu, TPM_BOTTOMALIGN, TPM_RIGHTBUTTON,
            WM_NULL,
        };
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SetForegroundWindow(frame.hwnd());
            TrackPopupMenu(
                self.inner.hmenu(),
                TPM_RIGHTBUTTON | TPM_BOTTOMALIGN,
                x,
                y,
                0,
                frame.hwnd(),
                std::ptr::null(),
            );
            PostMessageW(frame.hwnd(), WM_NULL, 0, 0);
        }
    }

    /// Non-Windows stub.
    #[cfg(not(target_os = "windows"))]
    pub fn popup_at(&self, _frame: &Frame, _x: i32, _y: i32) {}

    /// Borrow the underlying [`Menu`].
    pub fn as_menu(&self) -> &Menu {
        &self.inner
    }

    /// Mutably borrow the underlying [`Menu`].
    pub fn as_menu_mut(&mut self) -> &mut Menu {
        &mut self.inner
    }
}

impl Default for PopupMenu {
    fn default() -> Self {
        Self::new()
    }
}
