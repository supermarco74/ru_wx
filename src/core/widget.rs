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
use crate::platform::NativeHandle;
use std::cell::RefCell;
use std::rc::Rc;

/// Type alias for the native window handle used on Windows.
#[cfg(target_os = "windows")]
pub type Hwnd = NativeHandle;

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

    /// Screen-space bounds of the widget (top-left origin), read from the
    /// native backend through [`Widget::native_handle`].
    ///
    /// Unlike [`Widget::rect`], which is relative to the parent, this is
    /// the absolute position on the virtual desktop — the form used by
    /// the debug inspector. The default implementation delegates to the
    /// platform dispatcher, so concrete widgets need not override it. On
    /// macOS it must be called on the main thread.
    fn bounds_screen(&self) -> Rect {
        crate::platform::widget_screen_rect(self.native_handle())
    }

    /// Testo del tooltip mostrato al passaggio del mouse sopra il widget.
    ///
    /// È l'API di comodo per-elemento: ogni controllo della libreria
    /// (`Button`, `TextCtrl`, …) la eredita dal trait. L'implementazione
    /// di default instrada attraverso il backend tooltip condiviso
    /// (`tooltips_class32` su Windows, `NSView.toolTip` su macOS,
    /// `gtk_widget_set_tooltip_text` su Linux), quindi i widget concreti
    /// non devono fare override. Chiamarla di nuovo sostituisce il testo.
    fn set_tooltip(&self, text: &str) {
        crate::core::tooltip::attach_tooltip_to_handle(self.native_handle(), text);
    }
}

/// Shared reference to a widget (platform-independent)
pub type WidgetRef = Rc<RefCell<dyn Widget>>;

/// Abstract parent window. Any native window that can host child controls
/// implements this — currently `Frame` and `Panel`. Widgets that take a
/// parent are generic over `W: Window` so they can be created as a child
/// of either.
pub trait Window {
    /// Native top-level or container window handle ([`NativeHandle`]).
    fn hwnd(&self) -> NativeHandle;
}
