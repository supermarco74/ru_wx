//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
use std::cell::RefCell;
use std::rc::Rc;

use crate::window::frame::Frame;
use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};

#[cfg(target_os = "windows")]
use crate::platform::win32::{next_control_id, read_window_text, to_wide};
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

/// Win32 Button style constant
#[cfg(target_os = "windows")]
const BS_AUTOCHECKBOX: u32 = 0x0003;

struct CheckBoxInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    label: String,
    rect: Rect,
    enabled: bool,
    visible: bool,
}

#[derive(Clone)]
pub struct CheckBox {
    inner: Rc<RefCell<CheckBoxInner>>,
}

impl CheckBox {
    /// Create a new checkbox as a child of the given parent window.
    pub fn new<W: Window>(parent_in: &W, label: &str) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = parent_in.hwnd();
            let wide_label = to_wide(label);
            let wide_class = to_wide("BUTTON");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                wide_label.as_ptr(),
                WS_CHILD | WS_VISIBLE | BS_AUTOCHECKBOX,
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
        let _ = parent_in;

        CheckBox {
            inner: Rc::new(RefCell::new(CheckBoxInner {
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

    /// Returns true if the checkbox is currently checked
    pub fn is_checked(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let result = unsafe { SendMessageW(self.inner.borrow().hwnd, BM_GETCHECK, 0, 0) };
            result == BST_CHECKED_VALUE
        }

        #[cfg(not(target_os = "windows"))]
        false
    }

    /// Set the checked state of the checkbox
    pub fn set_checked(&self, checked: bool) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(
                self.inner.borrow().hwnd,
                BM_SETCHECK,
                if checked { BST_CHECKED } else { BST_UNCHECKED },
                0,
            );
        }
    }

    /// Register a callback that fires when the checkbox is toggled
    pub fn on_toggle<F: FnMut() + 'static>(&self, frame: &Frame, callback: F) {
        let id = self.inner.borrow().id;
        frame.register_command_handler(id, Box::new(callback));
    }

    /// Toggle with [`CheckBoxEvent`] payload (`wxCheckBoxEvent`).
    pub fn on_check_event<F: FnMut(&crate::CheckBoxEvent) + 'static>(
        &self,
        frame: &Frame,
        mut f: F,
    ) {
        let ctrl = self.clone();
        self.on_toggle(frame, move || {
            f(&crate::CheckBoxEvent::new(ctrl.is_checked()));
        });
    }

    /// Set the checkbox label text
    pub fn set_label(&self, label: &str) {
        self.inner.borrow_mut().label = label.to_string();
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(label);
            SetWindowTextW(self.inner.borrow().hwnd, wide.as_ptr());
        }
    }

    /// Get the current checkbox label.
    ///
    /// On Windows this queries the underlying button via
    /// `GetWindowTextW`, so it returns the live label.
    pub fn get_label(&self) -> String {
        #[cfg(target_os = "windows")]
        {
            read_window_text(self.inner.borrow().hwnd)
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.inner.borrow().label.clone()
        }
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

impl Widget for CheckBoxInner {
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
