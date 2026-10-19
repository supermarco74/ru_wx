//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Platform-specific backends for the UI toolkit.

#[cfg(target_os = "macos")]
pub mod appkit;
pub mod appkit_stubs;
#[cfg(target_os = "windows")]
pub mod default_api;
#[cfg(target_os = "linux")]
pub mod gtk;
pub mod gtk_stubs;
pub mod ids;
/// Cross-platform shell services (drives, recycle bin, DPI hit-tests,
/// DPAPI-style secrets, shell icons…). Dual-implementation: Win32 on
/// Windows, AppKit/POSIX on macOS, best-effort elsewhere.
pub mod shell;
pub mod stub_backend;

// ── Debug inspector — Livello 3 (schema condiviso + primitive native) ──
//
// Attivo solo in debug o con la feature `inspector` esplicita
// (cfr. PIANO_DEBUG_INSPECTOR.md §4). Il codice OS-specifico vive nei
// moduli `inspector_macos` / `inspector_windows`; l'inspector (Livello 1,
// in `src/debug/`) non importa mai `objc2` / `windows-sys`.
/// Complemento dell'inspector **sempre compilato**: registro dei widget
/// puro Rust (ruolo/stato dichiarati), usato dai container senza classe
/// nativa propria e letto dagli `inspector_*` sopra.
pub(crate) mod declared;
#[cfg(any(debug_assertions, feature = "inspector"))]
pub mod inspector;
#[cfg(all(target_os = "linux", any(debug_assertions, feature = "inspector")))]
pub mod inspector_gtk;
#[cfg(all(target_os = "macos", any(debug_assertions, feature = "inspector")))]
pub mod inspector_macos;
#[cfg(all(target_os = "windows", any(debug_assertions, feature = "inspector")))]
pub mod inspector_windows;

/// Opaque native handle (`HWND` on Windows, stub id on macOS/Linux).
/// Every public `hwnd()`, `handle()`, and `hbitmap` field uses this type
/// so caller code stays identical across targets.
pub type NativeHandle = isize;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::HMENU;

/// Extension trait for [`NativeHandle`] to support `is_null()` checks.
#[cfg(target_os = "windows")]
pub trait NativeHandleExt {
    /// Returns true if the native handle is null/zero (no valid window).
    fn is_null(&self) -> bool;
}

#[cfg(target_os = "windows")]
impl NativeHandleExt for isize {
    fn is_null(&self) -> bool {
        *self == 0
    }
}

/// Extension trait for `u32` to support DPI scaling via `.scale(dpi)`.
#[cfg(target_os = "windows")]
pub trait ScaleByDpi {
    /// Scale a width/height value by DPI. Used for HiDPI rendering.
    fn scale(self, dpi: u32) -> u32;
}

#[cfg(target_os = "windows")]
impl ScaleByDpi for u32 {
    fn scale(self, dpi: u32) -> u32 {
        (self as f64 * dpi as f64 / 96.0).round() as u32
    }
}

/// Cast a [`NativeHandle`] from [`crate::core::widget::Window::hwnd`] to
/// the Win32 `HWND` expected by FFI calls. Windows-only.
#[cfg(target_os = "windows")]
#[inline]
pub fn as_hwnd(handle: NativeHandle) -> HWND {
    handle as HWND
}

/// Cast a [`NativeHandle`] to the Win32 `HMENU` expected by FFI calls.
/// Windows-only.
#[cfg(target_os = "windows")]
#[inline]
pub fn as_hmenu(handle: NativeHandle) -> HMENU {
    handle as HMENU
}

/// Borrow the native window handle from any [`crate::core::widget::Window`].
#[inline]
pub fn window_handle<W: crate::core::widget::Window>(window: &W) -> NativeHandle {
    window.hwnd()
}

#[cfg(target_os = "windows")]
#[inline]
pub fn window_handle_as_hwnd<W: crate::core::widget::Window>(window: &W) -> HWND {
    as_hwnd(window.hwnd())
}

/// Screen-space rect (top-left origin) of a widget, given its native
/// handle.
///
/// Thin dispatcher over the per-OS backend primitive; it exists so that
/// [`crate::core::widget::Widget::bounds_screen`] can be provided as a
/// default trait method without each concrete widget reimplementing it.
///
/// Returns an all-zero rect on unsupported targets or for handles that
/// are not live on-screen widgets. On macOS this must be called on the
/// main thread.
pub fn widget_screen_rect(handle: NativeHandle) -> crate::core::geometry::Rect {
    #[cfg(target_os = "macos")]
    {
        appkit::mac_view_frame_rect(handle)
    }
    #[cfg(target_os = "windows")]
    {
        win32::window_rect(as_hwnd(handle))
    }
    #[cfg(target_os = "linux")]
    {
        gtk::widget_screen_rect(handle)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = handle;
        crate::core::geometry::Rect::new(0, 0, 0, 0)
    }
}

#[cfg(target_os = "windows")]
pub mod win32;
#[cfg(target_os = "windows")]
#[path = "win32_aui.rs"]
pub mod win32_aui;
#[cfg(target_os = "windows")]
pub mod win32_proprietary;
#[cfg(any(target_os = "windows", target_os = "macos"))]
pub mod window_icon;
#[cfg(target_os = "windows")]
pub mod winui3;

/// WinUI3 native using WinRT/XAML APIs (Microsoft.UI.Xaml)
/// This is a NEW separate backend from winui3 - uses XAML instead of Win32 controls
#[cfg(all(target_os = "windows", feature = "winui3_xaml"))]
pub mod winui3_xaml;

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

pub use ids::{next_control_id, next_menu_id, next_timer_id};

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
