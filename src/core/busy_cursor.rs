//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Hourglass cursor RAII (`wxBusyCursor`).

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{LoadCursorW, SetCursor, IDC_WAIT};

#[cfg(target_os = "macos")]
use std::cell::Cell;

#[cfg(target_os = "macos")]
thread_local! {
    static BUSY_DEPTH: Cell<u32> = const { Cell::new(0) };
}

/// Shows the wait cursor while in scope (`wxBusyCursor`).
pub struct BusyCursor {
    #[cfg(target_os = "windows")]
    prev: isize,
    #[cfg(target_os = "macos")]
    cursor: Option<objc2::rc::Retained<objc2_app_kit::NSCursor>>,
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
        #[cfg(target_os = "macos")]
        {
            let depth = BUSY_DEPTH.with(|d| {
                let n = d.get();
                d.set(n + 1);
                n
            });
            if depth == 0 {
                let cursor = objc2_app_kit::NSCursor::operationNotAllowedCursor();
                cursor.push();
                Self {
                    cursor: Some(cursor),
                }
            } else {
                Self { cursor: None }
            }
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        Self {}
    }
}

impl Drop for BusyCursor {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        unsafe {
            SetCursor(self.prev as _);
        }
        #[cfg(target_os = "macos")]
        {
            if let Some(cursor) = self.cursor.take() {
                cursor.pop();
            }
            BUSY_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
        }
    }
}
