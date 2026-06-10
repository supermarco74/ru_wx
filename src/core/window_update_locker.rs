//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Freeze window repaints (`wxWindowUpdateLocker`).

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{SendMessageW, WM_SETREDRAW};

#[cfg(target_os = "windows")]
use crate::core::widget::Window;

/// Suppresses window updates until dropped (`wxWindowUpdateLocker`).
pub struct WindowUpdateLocker {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    active: bool,
}

impl WindowUpdateLocker {
    /// Freeze repaints for `window` until this locker is dropped
    /// or [`WindowUpdateLocker::unlock`] is called.
    #[cfg(target_os = "windows")]
    pub fn new<W: Window>(window: &W) -> Self {
        let hwnd = window.hwnd();
        // SAFETY: WM_SETREDRAW FALSE pauses repaints; the HWND comes
        // from a live `Window` implementor so it is valid here.
        unsafe {
            SendMessageW(hwnd, WM_SETREDRAW, 0, 0);
        }
        Self { hwnd, active: true }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn new() -> Self {
        Self { active: true }
    }

    pub fn unlock(mut self) {
        self.release();
        std::mem::forget(self);
    }
}

impl Drop for WindowUpdateLocker {
    fn drop(&mut self) {
        self.release();
    }
}

impl WindowUpdateLocker {
    fn release(&mut self) {
        if !self.active {
            return;
        }
        #[cfg(target_os = "windows")]
        unsafe {
            SendMessageW(self.hwnd, WM_SETREDRAW, 1, 0);
        }
        self.active = false;
    }
}
