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
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[cfg(target_os = "windows")]
const BS_COMMANDLINK: u32 = 0x0000_000E;

struct CommandLinkButtonInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    main: String,
    note: String,
    rect: Rect,
    visible: bool,
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
                parent.hwnd(),
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = (parent, &label);

        Self {
            inner: Rc::new(RefCell::new(CommandLinkButtonInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                main: main.to_string(),
                note: note.to_string(),
                rect: Rect::new(0, 0, 260, 48),
                visible: true,
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
