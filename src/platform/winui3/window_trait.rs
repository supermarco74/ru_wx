//! `crate::core::widget::Window` and `Widget` trait implementations for
//! the WinUI3 backend.
//!
//! These impls live in a separate file so they can be linked **only**
//! when the `winui3` feature is enabled, avoiding namespace clashes
//! with the crate-root Win32 impls.

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::geometry::Rect;

use super::app::Frame;
use crate::core::widget::{Widget, WidgetRef};

impl crate::core::widget::Window for Frame {
    fn hwnd(&self) -> crate::platform::NativeHandle {
        // Reuse the existing `hwnd_native()` accessor to avoid duplicating
        // the HWND cast logic in two places.
        self.hwnd_native()
    }
}

/// Helper to produce a [`WidgetRef`] that wraps an HWND without
/// exposing the WndProc machinery. Used by every widget that needs
/// to be added to a [`BoxSizer`](super::layout::BoxSizer).
pub fn make_widget_ref(hwnd: windows_sys::Win32::Foundation::HWND) -> WidgetRef {
    struct W(windows_sys::Win32::Foundation::HWND);
    impl Widget for W {
        fn native_handle(&self) -> isize {
            self.0 as isize
        }

        fn set_position(&mut self, x: i32, y: i32) {
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos(
                    self.0,
                    std::ptr::null_mut(),
                    x,
                    y,
                    0,
                    0,
                    0x0010 | 0x0040, // SWP_NOSIZE | SWP_NOZORDER
                );
            }
        }

        fn set_size(&mut self, w: u32, h: u32) {
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos(
                    self.0,
                    std::ptr::null_mut(),
                    0,
                    0,
                    w as i32,
                    h as i32,
                    0x0002 | 0x0040, // SWP_NOMOVE | SWP_NOZORDER
                );
            }
        }

        fn rect(&self) -> Rect {
            unsafe {
                let mut r = std::mem::zeroed();
                windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect(self.0, &mut r);
                Rect {
                    x: r.left,
                    y: r.top,
                    width: (r.right - r.left) as u32,
                    height: (r.bottom - r.top) as u32,
                }
            }
        }

        fn is_visible(&self) -> bool {
            true
        }

        fn set_visible(&mut self, _v: bool) {
            // No-op for now: the WinUI3 backend creates widgets with
            // the WS_VISIBLE flag already set in `CreateWindowExW`.
        }

        fn is_enabled(&self) -> bool {
            true
        }

        fn set_enabled(&mut self, _e: bool) {}
    }
    Rc::new(RefCell::new(W(hwnd)))
}
