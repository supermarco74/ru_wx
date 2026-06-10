//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Hourglass cursor RAII (`wxBusyCursor`).

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{LoadCursorW, SetCursor, IDC_WAIT};

/// Shows the wait cursor while in scope (`wxBusyCursor`).
pub struct BusyCursor {
    #[cfg(target_os = "windows")]
    prev: isize,
}

impl Default for BusyCursor {
    fn default() -> Self {
        Self::new()
    }
}

impl BusyCursor {
    pub fn new() -> Self {
        #[cfg(target_os = "windows")]
        unsafe {
            let wait = LoadCursorW(std::ptr::null_mut(), IDC_WAIT);
            let prev = SetCursor(wait) as isize;
            Self { prev }
        }
        #[cfg(not(target_os = "windows"))]
        Self {}
    }
}

impl Drop for BusyCursor {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        unsafe {
            SetCursor(self.prev as _);
        }
    }
}
