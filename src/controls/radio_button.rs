//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
use std::cell::RefCell;
use std::rc::Rc;

use crate::window::frame::Frame;
use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};

#[cfg(target_os = "windows")]
use crate::platform::win32::{next_control_id, to_wide};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Win32 Button control messages
#[cfg(target_os = "windows")]
const BM_GETCHECK: u32 = 0x00F0;
#[cfg(target_os = "windows")]
const BM_SETCHECK: u32 = 0x00F1;

/// Win32 Button check state value for LRESULT comparison
#[cfg(target_os = "windows")]
const BST_CHECKED_VALUE: isize = 1;
/// Win32 Button check/uncheck WPARAM values for BM_SETCHECK
#[cfg(target_os = "windows")]
const BST_CHECKED: usize = 1;
#[cfg(target_os = "windows")]
const BST_UNCHECKED: usize = 0;

/// Win32 Button style constants
#[cfg(target_os = "windows")]
const BS_AUTORADIOBUTTON: u32 = 0x0009;

struct RadioButtonInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    #[allow(dead_code)]
    label: String,
    rect: Rect,
    enabled: bool,
    visible: bool,
}

#[derive(Clone)]
pub struct RadioButton {
    inner: Rc<RefCell<RadioButtonInner>>,
}

impl RadioButton {
    /// Create a new radio button as a child of the given frame.
    ///
    /// If `is_group_start` is true, the `WS_GROUP` style is added so that
    /// this radio button starts a new group (all subsequent radio buttons
    /// without `WS_GROUP` belong to the same group until the next group start).
    pub fn new<W: Window>(parent_in: &W, label: &str, is_group_start: bool) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = parent_in.hwnd();
            let wide_label = to_wide(label);
            let wide_class = to_wide("BUTTON");
            let style = if is_group_start {
                WS_CHILD | WS_VISIBLE | BS_AUTORADIOBUTTON | WS_GROUP
            } else {
                WS_CHILD | WS_VISIBLE | BS_AUTORADIOBUTTON
            };
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                wide_label.as_ptr(),
                style,
                0,
                0,
                120,
                24,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = (parent_in, is_group_start);

        RadioButton {
            inner: Rc::new(RefCell::new(RadioButtonInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                label: label.to_string(),
                rect: Rect::new(0, 0, 120, 24),
                enabled: true,
                visible: true,
            })),
        }
    }

    /// Returns true if this radio button is currently selected
    pub fn is_selected(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let result = unsafe { SendMessageW(self.inner.borrow().hwnd, BM_GETCHECK, 0, 0) };
            result == BST_CHECKED_VALUE
        }

        #[cfg(not(target_os = "windows"))]
        false
    }

    /// Set the selected state of the radio button
    pub fn set_selected(&self, selected: bool) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(
                self.inner.borrow().hwnd,
                BM_SETCHECK,
                if selected { BST_CHECKED } else { BST_UNCHECKED },
                0,
            );
        }
    }

    /// Register a callback that fires when the radio button is clicked/selected
    pub fn on_select<F: FnMut() + 'static>(&self, frame: &Frame, callback: F) {
        let id = self.inner.borrow().id;
        frame.register_command_handler(id, Box::new(callback));
    }

    /// Get the control ID
    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    /// Get a WidgetRef for use with sizers
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

impl Widget for RadioButtonInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.hwnd as isize
        }
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
