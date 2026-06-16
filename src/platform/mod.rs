//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Platform-specific backends for the UI toolkit.

pub mod appkit_stubs;
pub mod gtk_stubs;
pub mod ids;
pub mod stub_backend;

#[cfg(target_os = "windows")]
pub mod window_icon;
#[cfg(target_os = "windows")]
pub mod win32;

/// Active placeholder backend for the current target OS.
pub fn stub_backend_for_target() -> stub_backend::StubBackend {
    #[cfg(target_os = "macos")]
    {
        stub_backend::StubBackend::AppKit
    }
    #[cfg(not(target_os = "macos"))]
    {
        stub_backend::StubBackend::Gtk
    }
}

pub use ids::{next_control_id, next_menu_id};

/// Convert a Rust &str to a null-terminated wide string (UTF-16).
pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// Re-export the active platform's utilities so that user code can
// `use crate::platform::*` without needing to know which backend
// is in use.
#[cfg(target_os = "windows")]
pub use win32::get_device_caps_dpi;
#[cfg(target_os = "windows")]
pub use win32::read_window_text;
