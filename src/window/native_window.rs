//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Native window handle wrapper (`wxNativeWindow`).

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef};

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HWND;

struct NativeWindowInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    rect: Rect,
}

/// Embeds an existing native `HWND` (`wxNativeWindow`).
#[derive(Clone)]
pub struct NativeWindow {
    inner: Rc<RefCell<NativeWindowInner>>,
}

impl NativeWindow {
    #[cfg(target_os = "windows")]
    pub fn from_handle(hwnd: HWND, rect: Rect) -> Self {
        Self {
            inner: Rc::new(RefCell::new(NativeWindowInner { hwnd, rect })),
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn from_handle(_handle: isize, rect: Rect) -> Self {
        Self {
            inner: Rc::new(RefCell::new(NativeWindowInner { rect })),
        }
    }

    pub fn rect(&self) -> Rect {
        self.inner.borrow().rect
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

impl Widget for NativeWindowInner {
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
    }

    fn set_size(&mut self, w: u32, h: u32) {
        self.rect.width = w;
        self.rect.height = h;
    }

    fn rect(&self) -> Rect {
        self.rect
    }

    fn is_visible(&self) -> bool {
        true
    }

    fn set_visible(&mut self, _visible: bool) {}

    fn is_enabled(&self) -> bool {
        true
    }

    fn set_enabled(&mut self, _enabled: bool) {}
}
