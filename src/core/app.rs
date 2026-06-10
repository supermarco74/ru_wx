//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Application entry point and message-loop driver.
//!
//! [`App::new`] performs any platform-specific one-shot initialisation
//! (on Windows it calls `InitCommonControlsEx` so the common controls
//! render with visual styles) and [`App::run`] enters the platform
//! event loop on the supplied top-level frame, blocking until the
//! window is closed.
//!
//! ```no_run
//! use ru_wx::prelude::*;
//! let app = App::new();
//! let frame = Frame::builder().with_title("hi").build();
//! app.run(frame);
//! ```

use crate::window::frame::Frame;

/// Application entry point.
/// On Windows, this is a thin wrapper that calls frame.show() which runs the message loop.
/// On macOS, this would set up NSApplication.
/// On Linux, this would initialize GTK.
pub struct App;

impl App {
    pub fn new() -> Self {
        // Platform-specific initialization could go here
        // e.g., on Windows: InitCommonControlsEx for visual styles
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            use windows_sys::Win32::UI::Controls::*;
            let icc = INITCOMMONCONTROLSEX {
                dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
                // ICC_STANDARD_CLASSES already covers the tab control, but
                // we include ICC_TAB_CLASSES and ICC_LISTVIEW_CLASSES
                // explicitly to be belt-and-suspenders robust against any
                // `ICC_*` flag refactor in future `windows-sys` releases.
                dwICC: ICC_STANDARD_CLASSES
                    | ICC_BAR_CLASSES
                    | ICC_TAB_CLASSES
                    | ICC_LISTVIEW_CLASSES
                    | ICC_LINK_CLASS
                    | ICC_DATE_CLASSES,
            };
            InitCommonControlsEx(&icc);
        }
        App
    }

    /// Run the application with the given frame.
    /// This enters the platform event loop and does not return until the window is closed.
    pub fn run(self, frame: Frame) {
        frame.show();
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
