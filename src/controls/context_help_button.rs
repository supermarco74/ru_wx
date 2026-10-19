//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Context-help button (`wxContextHelpButton`).

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::context_help_event::ContextHelpEvent;
use crate::core::geometry::{Point, Rect};
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

struct ContextHelpButtonInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(target_os = "macos")]
    stub_handle: crate::platform::NativeHandle,
    #[cfg(target_os = "macos")]
    parent_handle: crate::platform::NativeHandle,
    id: u16,
    rect: Rect,
    visible: bool,
    enabled: bool,
}

/// Small "?" button that requests context help (`wxContextHelpButton`).
#[derive(Clone)]
pub struct ContextHelpButton {
    inner: Rc<RefCell<ContextHelpButtonInner>>,
}

impl ContextHelpButton {
    pub fn new<W: Window>(parent: &W) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        let hwnd = unsafe {
            let parent_hwnd = crate::platform::window_handle_as_hwnd(parent);
            let label = to_wide("?");
            let class = to_wide("BUTTON");
            CreateWindowExW(
                0,
                class.as_ptr(),
                label.as_ptr(),
                WS_CHILD | WS_VISIBLE | (BS_PUSHBUTTON as u32),
                0,
                0,
                24,
                24,
                parent_hwnd,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = parent;

        #[cfg(target_os = "macos")]
        let parent_handle = crate::platform::window_handle(parent);
        #[cfg(target_os = "macos")]
        let stub_handle =
            crate::platform::appkit::create_button(parent_handle, "?", id, Rect::new(0, 0, 24, 24));

        Self {
            inner: Rc::new(RefCell::new(ContextHelpButtonInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(target_os = "macos")]
                stub_handle,
                #[cfg(target_os = "macos")]
                parent_handle,
                id,
                rect: Rect::new(0, 0, 24, 24),
                visible: true,
                enabled: true,
            })),
        }
    }

    pub fn on_help<F: FnMut(&ContextHelpEvent) + 'static>(&self, frame: &Frame, mut f: F) {
        let id = self.inner.borrow().id;
        frame.register_command_handler(
            id,
            Box::new(move || {
                f(&ContextHelpEvent::new(id, Point::new(0, 0)));
            }),
        );
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

impl Widget for ContextHelpButtonInner {
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
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = enabled;
    }
}
