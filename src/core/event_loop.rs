//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Auxiliary event loop (`wxEventLoop`).

/// Nested or auxiliary message pump (`wxEventLoop`).
#[derive(Debug, Default)]
pub struct EventLoop {
    running: bool,
}

impl EventLoop {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Pump pending messages once (Win32). Returns `false` on quit.
    #[cfg(target_os = "windows")]
    pub fn dispatch_pending(&mut self) -> bool {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, PeekMessageW, TranslateMessage, PM_REMOVE, WM_QUIT,
        };

        self.running = true;
        let mut msg = std::mem::MaybeUninit::uninit();
        unsafe {
            if PeekMessageW(
                msg.as_mut_ptr(),
                HWND::default(),
                0,
                0,
                PM_REMOVE,
            ) == 0
            {
                return true;
            }
            let msg = msg.assume_init();
            if msg.message == WM_QUIT {
                self.running = false;
                return false;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        true
    }

    #[cfg(not(target_os = "windows"))]
    pub fn dispatch_pending(&mut self) -> bool {
        self.running
    }
}
