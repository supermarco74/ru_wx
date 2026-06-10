//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Platform-specific backends for the UI toolkit.
//!
//! Each supported operating system ships its own submodule
//! ([`win32`] is the only one today). Only the submodule that matches
//! the current build target is compiled; cross-platform code can
//! `use crate::platform::*` and get the active backend's helpers
//! transparently.
//!
//! Conventions:
//! * Functions that wrap a single Win32 FFI call live in the matching
//!   platform module and are re-exported here.
//! * Functions that return a handle are allowed to return a null
//!   pointer when the underlying call fails; callers are responsible
//!   for null-checking. The library also logs the failure through the
//!   [`crate::core::log`] system when appropriate.
//! * No function in this module panics. `try_into` / lock failures
//!   fall back to a sensible default (96 DPI, an empty string, etc.)
//!   rather than aborting the process.

pub mod appkit_stubs;
pub mod gtk_stubs;

#[cfg(target_os = "windows")]
pub mod win32;

// Future platforms:
//
// #[cfg(target_os = "macos")]
// pub mod macos;
//
// #[cfg(target_os = "linux")]
// pub mod linux;

// Re-export the active platform's utilities so that user code can
// `use crate::platform::*` without needing to know which backend
// is in use.
#[cfg(target_os = "windows")]
pub use win32::*;
