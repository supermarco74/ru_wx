//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Core widget traits and shared type-erased references.
//!
//! Every concrete control (`Button`, `StaticText`, …) implements the
//! [`Widget`] trait, which exposes only the platform-agnostic operations
//! (position, size, visibility, enabled, native handle). A
//! [`WidgetRef`] is the `Rc<RefCell<dyn Widget>>` alias used by sizers
//! to keep heterogeneous children without generics.
//!
//! On Windows the [`Window`] trait is a small extension trait over
//! anything that can hand out an `HWND` — most widgets implement it.

use crate::core::geometry::Rect;
use std::cell::RefCell;
use std::rc::Rc;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HWND;

/// Platform-independent widget trait.
/// Each platform backend implements this for its native controls.
pub trait Widget {
    /// Returns the platform-native handle (HWND on Windows, NSView* on macOS, GtkWidget* on Linux)
    /// Represented as a raw pointer-sized value for cross-platform compatibility.
    fn native_handle(&self) -> isize;

    fn set_position(&mut self, x: i32, y: i32);
    fn set_size(&mut self, w: u32, h: u32);
    fn rect(&self) -> Rect;
    fn is_visible(&self) -> bool;
    fn set_visible(&mut self, visible: bool);
    fn is_enabled(&self) -> bool;
    fn set_enabled(&mut self, enabled: bool);
}

/// Shared reference to a widget (platform-independent)
pub type WidgetRef = Rc<RefCell<dyn Widget>>;

/// Abstract parent window. Any native window that can host child controls
/// implements this — currently `Frame` and `Panel`. Widgets that take a
/// parent are generic over `W: Window` so they can be created as a child
/// of either.
#[cfg(target_os = "windows")]
pub trait Window {
    /// Return the platform-native window handle (HWND on Windows).
    fn hwnd(&self) -> HWND;
}

/// On non-Windows platforms this trait exposes the stub native handle (`isize`).
#[cfg(not(target_os = "windows"))]
pub trait Window {
    fn hwnd(&self) -> isize;
}
