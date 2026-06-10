//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Owner-drawn combo box (`wxOwnerDrawnComboBox`).
//!
//! Wraps a standard Win32 combo with `CBS_OWNERDRAWFIXED` for custom item painting.

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};
use crate::window::frame::Frame;

#[cfg(target_os = "windows")]
use crate::platform::win32::{next_control_id, to_wide};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[cfg(target_os = "windows")]
const CBS_OWNERDRAWFIXED: u32 = 0x0010;
#[cfg(target_os = "windows")]
const CBS_DROPDOWNLIST: u32 = 0x0003;
#[cfg(target_os = "windows")]
const CB_ADDSTRING: u32 = 0x0143;

struct OwnerDrawnComboBoxInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    rect: Rect,
    items: Vec<String>,
    visible: bool,
}

#[derive(Clone)]
pub struct OwnerDrawnComboBox {
    inner: Rc<RefCell<OwnerDrawnComboBoxInner>>,
}

impl OwnerDrawnComboBox {
    pub fn new<W: Window>(parent: &W) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        let hwnd = unsafe {
            let wide_class = to_wide("COMBOBOX");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | CBS_DROPDOWNLIST | CBS_OWNERDRAWFIXED,
                0,
                0,
                160,
                200,
                parent.hwnd(),
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = parent;

        Self {
            inner: Rc::new(RefCell::new(OwnerDrawnComboBoxInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                rect: Rect::new(0, 0, 160, 24),
                items: Vec::new(),
                visible: true,
            })),
        }
    }

    pub fn append(&self, label: &str) {
        self.inner.borrow_mut().items.push(label.to_string());
        #[cfg(target_os = "windows")]
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            let wide = to_wide(label);
            SendMessageW(hwnd, CB_ADDSTRING, 0, wide.as_ptr() as isize);
        }
    }

    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    pub fn on_selection_change(&self, frame: &Frame, mut f: impl FnMut(usize) + 'static) {
        frame.register_command_handler(self.id(), Box::new(move || f(0)));
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

impl Widget for OwnerDrawnComboBoxInner {
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
