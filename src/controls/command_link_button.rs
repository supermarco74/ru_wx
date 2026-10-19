//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Vista-style command link (`wxCommandLinkButton`).

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
const BS_COMMANDLINK: u32 = 0x0000_000E;

struct CommandLinkButtonInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(target_os = "macos")]
    stub_handle: crate::platform::NativeHandle,
    #[cfg(target_os = "macos")]
    parent_handle: crate::platform::NativeHandle,
    id: u16,
    main: String,
    note: String,
    rect: Rect,
    visible: bool,
    enabled: bool,
}

#[derive(Clone)]
pub struct CommandLinkButton {
    inner: Rc<RefCell<CommandLinkButtonInner>>,
}

impl CommandLinkButton {
    pub fn new<W: Window>(parent: &W, main: &str, note: &str) -> Self {
        let id = next_control_id();
        let label = format!("{main}\n{note}");

        #[cfg(target_os = "windows")]
        let hwnd = unsafe {
            let wide = to_wide(&label);
            let class = to_wide("BUTTON");
            CreateWindowExW(
                0,
                class.as_ptr(),
                wide.as_ptr(),
                WS_CHILD | WS_VISIBLE | BS_COMMANDLINK,
                0,
                0,
                260,
                48,
                crate::platform::window_handle_as_hwnd(parent),
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = parent;

        #[cfg(not(target_os = "windows"))]
        let _ = &label;

        #[cfg(target_os = "macos")]
        let parent_handle = crate::platform::window_handle(parent);
        #[cfg(target_os = "macos")]
        let stub_handle = crate::platform::appkit::create_command_link_button(
            parent_handle,
            main,
            note,
            id,
            Rect::new(0, 0, 260, 48),
        );

        Self {
            inner: Rc::new(RefCell::new(CommandLinkButtonInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(target_os = "macos")]
                stub_handle,
                #[cfg(target_os = "macos")]
                parent_handle,
                id,
                main: main.to_string(),
                note: note.to_string(),
                rect: Rect::new(0, 0, 260, 48),
                visible: true,
                enabled: true,
            })),
        }
    }

    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    pub fn on_click(&self, frame: &Frame, f: impl FnMut() + 'static) {
        frame.register_command_handler(self.id(), Box::new(f));
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

impl Widget for CommandLinkButtonInner {
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
