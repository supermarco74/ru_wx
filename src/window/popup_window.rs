//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Generic popup window (`wxPopupWindow`).

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::geometry::Rect;
use crate::window::frame::Frame;

#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[derive(Clone)]
pub struct PopupWindow {
    inner: Rc<RefCell<PopupWindowInner>>,
}

struct PopupWindowInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(target_os = "macos")]
    handle: crate::platform::NativeHandle,
    rect: Rect,
}

impl PopupWindow {
    /// Borderless popup child of `parent`.
    #[cfg(target_os = "windows")]
    pub fn new(parent: &Frame, rect: Rect) -> Self {
        let hwnd = unsafe {
            let class = to_wide("STATIC");
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
                class.as_ptr(),
                std::ptr::null(),
                WS_POPUP | WS_VISIBLE | WS_BORDER,
                rect.x,
                rect.y,
                rect.width.max(40) as i32,
                rect.height.max(24) as i32,
                crate::platform::window_handle_as_hwnd(parent),
                std::ptr::null_mut(),
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null_mut(),
            )
        };
        Self {
            inner: Rc::new(RefCell::new(PopupWindowInner { hwnd, rect })),
        }
    }

    #[cfg(target_os = "macos")]
    pub fn new(_parent: &Frame, rect: Rect) -> Self {
        let handle = crate::platform::appkit::create_borderless_popup(rect);
        Self {
            inner: Rc::new(RefCell::new(PopupWindowInner { handle, rect })),
        }
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    pub fn new(_parent: &Frame, rect: Rect) -> Self {
        Self {
            inner: Rc::new(RefCell::new(PopupWindowInner { rect })),
        }
    }

    pub fn close(&self) {
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            if !hwnd.is_null() {
                unsafe {
                    DestroyWindow(hwnd);
                }
                self.inner.borrow_mut().hwnd = std::ptr::null_mut();
            }
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::popup_close(self.inner.borrow().handle);
        }
    }

    #[cfg(target_os = "macos")]
    pub fn handle(&self) -> crate::platform::NativeHandle {
        self.inner.borrow().handle
    }

    #[cfg(target_os = "windows")]
    pub fn hwnd(&self) -> HWND {
        self.inner.borrow().hwnd
    }
}
