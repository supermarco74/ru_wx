//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
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

/// Crea il `gtk::CheckButton` (radio) nativo e restituisce (handle, genitore).
#[cfg(target_os = "linux")]
fn gtk_radio_button<W: Window>(
    parent_in: &W,
    label: &str,
    is_group_start: bool,
    id: u16,
) -> (isize, crate::platform::NativeHandle) {
    let parent = crate::platform::window_handle(parent_in);
    let handle = crate::platform::gtk::create_radio_button(
        parent,
        label,
        id,
        is_group_start,
        Rect::new(0, 0, 120, 24),
    );
    (handle, parent)
}

struct RadioButtonInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    stub_handle: isize,
    #[cfg(target_os = "macos")]
    stub_handle: isize,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    parent_handle: crate::platform::NativeHandle,
    id: u16,
    #[allow(dead_code)]
    label: String,
    rect: Rect,
    enabled: bool,
    visible: bool,
    #[cfg(all(
        not(target_os = "windows"),
        not(target_os = "macos"),
        not(target_os = "linux")
    ))]
    selected: bool,
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
            let parent = crate::platform::window_handle_as_hwnd(parent_in);
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

        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        let _ = (parent_in, is_group_start);

        #[cfg(target_os = "macos")]
        let parent_handle = crate::platform::window_handle(parent_in);
        #[cfg(target_os = "macos")]
        let stub_handle = crate::platform::appkit::create_radio_button(
            parent_handle,
            label,
            id,
            Rect::new(0, 0, 120, 24),
        );
        #[cfg(target_os = "macos")]
        let _ = is_group_start;
        #[cfg(target_os = "linux")]
        let (stub_handle, parent_handle) = gtk_radio_button(parent_in, label, is_group_start, id);

        RadioButton {
            inner: Rc::new(RefCell::new(RadioButtonInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(all(
                    not(target_os = "windows"),
                    not(target_os = "macos"),
                    not(target_os = "linux")
                ))]
                stub_handle: crate::platform::stub_backend::alloc_widget_handle(),
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                stub_handle,
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                parent_handle,
                id,
                label: label.to_string(),
                rect: Rect::new(0, 0, 120, 24),
                enabled: true,
                visible: true,
                #[cfg(all(
                    not(target_os = "windows"),
                    not(target_os = "macos"),
                    not(target_os = "linux")
                ))]
                selected: false,
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

        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::radio_is_selected(self.inner.borrow().stub_handle)
        }

        #[cfg(target_os = "linux")]
        {
            let handle = self.inner.borrow().stub_handle;
            crate::platform::gtk::radio_button_is_selected(handle)
        }

        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        {
            self.inner.borrow().selected
        }
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

        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::radio_set_selected(self.inner.borrow().stub_handle, selected);
        }

        #[cfg(target_os = "linux")]
        {
            // Handle estratto prima della chiamata: GTK emette
            // `toggled` in modo sincrono e un `borrow()` vivo su
            // `inner` causerebbe re-entrancy (panic RefCell).
            let handle = self.inner.borrow().stub_handle;
            crate::platform::gtk::radio_button_set_selected(handle, selected);
        }

        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        {
            self.inner.borrow_mut().selected = selected;
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
        #[cfg(not(target_os = "windows"))]
        {
            self.stub_handle
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
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::set_view_frame(
                self.parent_handle,
                self.stub_handle,
                self.rect,
            );
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_view_frame(self.parent_handle, self.stub_handle, self.rect);
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
            crate::platform::appkit::set_view_frame(
                self.parent_handle,
                self.stub_handle,
                self.rect,
            );
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_view_frame(self.parent_handle, self.stub_handle, self.rect);
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
            crate::platform::appkit::set_view_visible(self.stub_handle, visible);
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_view_visible(self.stub_handle, visible);
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
            crate::platform::appkit::set_view_enabled(self.stub_handle, enabled);
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_view_enabled(self.stub_handle, enabled);
        }
    }
}
