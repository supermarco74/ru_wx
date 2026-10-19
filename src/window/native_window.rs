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
    #[cfg(target_os = "macos")]
    view_handle: crate::platform::NativeHandle,
    #[cfg(target_os = "macos")]
    parent_handle: crate::platform::NativeHandle,
    rect: Rect,
    enabled: bool,
}

/// Embeds an existing native window handle (`wxNativeWindow`).
#[derive(Clone)]
pub struct NativeWindow {
    inner: Rc<RefCell<NativeWindowInner>>,
}

impl NativeWindow {
    /// Wrap an existing native window handle.
    ///
    /// The accepted type is [`crate::platform::NativeHandle`] (a
    /// platform-agnostic `isize`-shaped alias of `HWND` on Windows),
    /// so callers can pass the result of [`crate::platform::window_handle`]
    /// or a literal `0` from any target without platform-specific casts.
    #[cfg(target_os = "windows")]
    pub fn from_handle(hwnd: crate::platform::NativeHandle, rect: Rect) -> Self {
        Self {
            inner: Rc::new(RefCell::new(NativeWindowInner {
                hwnd: hwnd as HWND,
                rect,
                enabled: true,
            })),
        }
    }

    #[cfg(target_os = "macos")]
    pub fn from_handle(
        view_handle: crate::platform::NativeHandle,
        parent: &impl crate::core::widget::Window,
        rect: Rect,
    ) -> Self {
        let parent_handle = crate::platform::window_handle(parent);
        crate::platform::appkit::embed_native_view(parent_handle, view_handle, rect);
        Self {
            inner: Rc::new(RefCell::new(NativeWindowInner {
                view_handle,
                parent_handle,
                rect,
                enabled: true,
            })),
        }
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    pub fn from_handle(_handle: isize, rect: Rect) -> Self {
        Self {
            inner: Rc::new(RefCell::new(NativeWindowInner {
                rect,
                enabled: true,
            })),
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
        #[cfg(target_os = "macos")]
        {
            self.view_handle as isize
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            0
        }
    }

    fn set_position(&mut self, x: i32, y: i32) {
        self.rect.x = x;
        self.rect.y = y;
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::macos_widget_set_frame(
                self.parent_handle,
                self.view_handle,
                self.rect,
            );
        }
    }

    fn set_size(&mut self, w: u32, h: u32) {
        self.rect.width = w;
        self.rect.height = h;
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::macos_widget_set_frame(
                self.parent_handle,
                self.view_handle,
                self.rect,
            );
        }
    }

    fn rect(&self) -> Rect {
        self.rect
    }

    fn is_visible(&self) -> bool {
        true
    }

    fn set_visible(&mut self, visible: bool) {
        #[cfg(target_os = "macos")]
        crate::platform::appkit::macos_widget_set_visible(self.view_handle, visible);
        #[cfg(not(target_os = "macos"))]
        let _ = visible;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::macos_widget_set_enabled(self.view_handle, enabled);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = enabled;
    }
}
