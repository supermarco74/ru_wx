//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Modal window blocker (`wxWindowDisabler`).

use crate::window::frame::Frame;

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;

/// Disables a top-level window until dropped (`wxWindowDisabler`).
pub struct WindowDisabler {
    #[cfg(target_os = "windows")]
    hwnd: windows_sys::Win32::Foundation::HWND,
}

impl WindowDisabler {
    pub fn new(frame: &Frame) -> Self {
        #[cfg(target_os = "windows")]
        {
            let hwnd = frame.hwnd();
            unsafe {
                EnableWindow(hwnd, 0);
            }
            Self { hwnd }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = frame;
            Self {}
        }
    }
}

impl Drop for WindowDisabler {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        unsafe {
            EnableWindow(self.hwnd, 1);
        }
    }
}
