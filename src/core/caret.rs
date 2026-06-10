//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Text caret (`wxCaret`) for custom editors on Win32.

use crate::core::widget::Window;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateCaret, DestroyCaret, HideCaret, SetCaretPos, ShowCaret,
};

/// A screen caret attached to a window's client area.
pub struct Caret {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    width: i32,
    height: i32,
    #[cfg(target_os = "windows")]
    created: bool,
}

impl Caret {
    /// Create a caret for `window` (not shown until [`Self::show`]).
    #[cfg(target_os = "windows")]
    pub fn new<W: Window>(window: &W, width: i32, height: i32) -> Self {
        let hwnd = window.hwnd();
        Self {
            hwnd,
            width,
            height,
            created: false,
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn new<W: Window>(_window: &W, width: i32, height: i32) -> Self {
        Self { width, height }
    }

    /// Allocate the OS caret and show it at `(x, y)`.
    #[cfg(target_os = "windows")]
    pub fn show(&mut self, x: i32, y: i32) -> bool {
        unsafe {
            if !self.created {
                if CreateCaret(self.hwnd, 0 as _, self.width, self.height) == 0 {
                    return false;
                }
                self.created = true;
            }
            SetCaretPos(x, y) != 0 && ShowCaret(self.hwnd) != 0
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn show(&mut self, _x: i32, _y: i32) -> bool {
        false
    }

    /// Hide the caret without destroying it.
    #[cfg(target_os = "windows")]
    pub fn hide(&self) {
        if self.created {
            unsafe {
                HideCaret(self.hwnd);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn hide(&self) {}

    /// Move the caret (must be visible or created).
    #[cfg(target_os = "windows")]
    pub fn set_position(&self, x: i32, y: i32) -> bool {
        if self.created {
            unsafe { SetCaretPos(x, y) != 0 }
        } else {
            false
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn set_position(&self, _x: i32, _y: i32) -> bool {
        false
    }
}

#[cfg(target_os = "windows")]
impl Drop for Caret {
    fn drop(&mut self) {
        if self.created {
            unsafe {
                DestroyCaret();
            }
        }
    }
}
