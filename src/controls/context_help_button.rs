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
use windows_sys::Win32::UI::WindowsAndMessaging::*;

struct ContextHelpButtonInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    rect: Rect,
    visible: bool,
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
            let parent_hwnd = parent.hwnd();
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

        #[cfg(not(target_os = "windows"))]
        let _ = parent;

        Self {
            inner: Rc::new(RefCell::new(ContextHelpButtonInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                rect: Rect::new(0, 0, 24, 24),
                visible: true,
            })),
        }
    }

    pub fn on_help<F: FnMut(&ContextHelpEvent) + 'static>(&self, frame: &Frame, mut f: F) {
        let id = self.inner.borrow().id;
        frame.register_command_handler(id, Box::new(move || {
            f(&ContextHelpEvent::new(id, Point::new(0, 0)));
        }));
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
        #[cfg(not(target_os = "windows"))]
        {
            0
        }
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
    }

    fn set_size(&mut self, w: u32, h: u32) {
        self.rect.width = w;
        self.rect.height = h;
        #[cfg(target_os = "windows")]
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
        unsafe {
            ShowWindow(self.hwnd, if visible { SW_SHOW } else { SW_HIDE });
        }
    }

    fn is_enabled(&self) -> bool {
        true
    }

    fn set_enabled(&mut self, _enabled: bool) {}
}
