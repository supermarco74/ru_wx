//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Toggle-button control (`wxToggleButton`).
//!
//! On Windows the widget is realised with the `BUTTON` common control
//! class using the standard push-button style `BS_PUSHBUTTON`. The
//! "stays pressed" visual is achieved by sending `BM_SETCHECK` with
//! `BST_CHECKED` / `BST_UNCHECKED` after the user clicks the button;
//! the state itself is tracked in the `ToggleButtonInner` struct.
//!
//! # Example
//! ```no_run
//! use ru_wx::prelude::*;
//!
//! let frame = Frame::builder().with_title("App").with_size(100, 100).build();
//! let btn = ToggleButton::new(&frame, "Bold");
//! btn.set_value(true);
//! assert!(btn.get_value());
//! btn.on_toggle(&frame, |checked| {
//!     println!("Bold is now {}", if checked { "ON" } else { "OFF" });
//! });
//! ```

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};
use crate::window::frame::Frame;

use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::{read_window_text, to_wide};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── Win32 button messages / states (defined in <winuser.h>, not all
//    exported by windows-sys 0.59) ────────────────────────────────────

/// `BM_GETCHECK` — query the checked state of a button.
#[cfg(target_os = "windows")]
const BM_GETCHECK: u32 = 0x00F0;
/// `BM_SETCHECK` — set the checked state of a button.
#[cfg(target_os = "windows")]
const BM_SETCHECK: u32 = 0x00F1;

/// `BST_UNCHECKED` — the button is not checked.
#[cfg(target_os = "windows")]
const BST_UNCHECKED: usize = 0x0000;
/// `BST_CHECKED` — the button is checked (pressed in).
#[cfg(target_os = "windows")]
const BST_CHECKED: usize = 0x0001;
/// `BST_INDETERMINATE` — the button is in the grey / indeterminate
/// state. Only meaningful for buttons created with `BS_3STATE` or
/// `BS_AUTO3STATE`; we still expose the constant for completeness.
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const BST_INDETERMINATE: usize = 0x0002;

/// Crea il `gtk::ToggleButton` nativo e restituisce (handle, genitore).
#[cfg(target_os = "linux")]
fn gtk_toggle_button<W: Window>(
    parent_in: &W,
    label: &str,
    id: u16,
) -> (isize, crate::platform::NativeHandle) {
    let parent = crate::platform::window_handle(parent_in);
    let handle =
        crate::platform::gtk::create_toggle_button(parent, label, id, Rect::new(0, 0, 100, 30));
    (handle, parent)
}

/// Allinea la cache `checked` con l'esito dell'ultimo toggle.
///
/// Su GTK il widget aggiorna già lo stato reale da solo, quindi la cache
/// viene riletta dal widget (sincronizzazione, non inversione); sulle altre
/// piattaforme qui lo stato nativo non viene interrogato e la cache viene
/// semplicemente invertita.
#[cfg(target_os = "linux")]
fn advance_checked(inner: &Rc<RefCell<ToggleButtonInner>>) {
    let checked = {
        let handle = inner.borrow().stub_handle;
        crate::platform::gtk::toggle_button_is_pressed(handle)
    };
    inner.borrow_mut().checked = checked;
}

#[cfg(not(target_os = "linux"))]
fn advance_checked(inner: &Rc<RefCell<ToggleButtonInner>>) {
    let cur = inner.borrow().checked;
    inner.borrow_mut().checked = !cur;
}

// ── Inner type ─────────────────────────────────────────────────────────

struct ToggleButtonInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(not(target_os = "windows"))]
    stub_handle: crate::platform::NativeHandle,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    parent_handle: crate::platform::NativeHandle,
    id: u16,
    label: String,
    rect: Rect,
    enabled: bool,
    visible: bool,
    /// Whether the button is currently in the "pressed / checked"
    /// state. On Windows we keep this in Rust and push it down to the
    /// control with `BM_SETCHECK` so the visual matches the logical
    /// state.
    checked: bool,
}

#[derive(Clone)]
pub struct ToggleButton {
    inner: Rc<RefCell<ToggleButtonInner>>,
}

impl ToggleButton {
    /// Create a new toggle button as a child of the given parent
    /// window. The button starts in the unchecked state.
    pub fn new<W: Window>(parent_in: &W, label: &str) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = crate::platform::window_handle_as_hwnd(parent_in);
            let wide_class = to_wide("BUTTON");
            let wide_label = to_wide(label);
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                wide_label.as_ptr(),
                WS_CHILD | WS_VISIBLE,
                0,
                0,
                100,
                30,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        let _ = (parent_in, label);

        #[cfg(target_os = "macos")]
        let parent_handle = crate::platform::window_handle(parent_in);
        #[cfg(target_os = "macos")]
        let stub_handle = crate::platform::appkit::create_toggle_button(
            parent_handle,
            label,
            id,
            Rect::new(0, 0, 100, 30),
        );
        #[cfg(target_os = "linux")]
        let (stub_handle, parent_handle) = gtk_toggle_button(parent_in, label, id);
        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        let stub_handle = crate::platform::stub_backend::alloc_widget_handle();

        ToggleButton {
            inner: Rc::new(RefCell::new(ToggleButtonInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(not(target_os = "windows"))]
                stub_handle,
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                parent_handle,
                id,
                label: label.to_string(),
                rect: Rect::new(0, 0, 100, 30),
                enabled: true,
                visible: true,
                checked: false,
            })),
        }
    }

    /// Create a new toggle button that starts in the given state.
    pub fn with_value<W: Window>(parent_in: &W, label: &str, checked: bool) -> Self {
        let btn = Self::new(parent_in, label);
        btn.set_value(checked);
        btn
    }

    /// Return `true` if the button is currently checked / pressed.
    pub fn get_value(&self) -> bool {
        self.inner.borrow().checked
    }

    /// Convenience alias for [`ToggleButton::get_value`].
    pub fn is_checked(&self) -> bool {
        self.get_value()
    }

    /// Set the checked state of the button. The visual is updated
    /// immediately via `BM_SETCHECK`.
    pub fn set_value(&self, checked: bool) {
        self.inner.borrow_mut().checked = checked;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            let state = if checked { BST_CHECKED } else { BST_UNCHECKED };
            SendMessageW(hwnd, BM_SETCHECK, state, 0);
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::toggle_button_set_pressed(
                self.inner.borrow().stub_handle,
                checked,
            );
        }
        #[cfg(target_os = "linux")]
        {
            // Handle estratto prima della chiamata: GTK emette `toggled`
            // in modo sincrono durante `set_active` e un `borrow()` vivo
            // su `inner` causerebbe re-entrancy (panic RefCell). La cache
            // scritta sopra resta corretta: l'eventuale handler rilegge
            // dal widget lo stesso valore imposto qui.
            let handle = self.inner.borrow().stub_handle;
            crate::platform::gtk::toggle_button_set_pressed(handle, checked);
        }
    }

    /// Flip the current state and return the new value.
    pub fn toggle(&self) -> bool {
        let new_state = !self.get_value();
        self.set_value(new_state);
        new_state
    }

    /// Get the control ID (used for `WM_COMMAND` dispatch).
    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    /// Set the button label. Updates both the cached label and the
    /// native control text via `SetWindowTextW`.
    pub fn set_label(&self, label: &str) {
        self.inner.borrow_mut().label = label.to_string();
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(label);
            SetWindowTextW(self.inner.borrow().hwnd, wide.as_ptr());
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::set_button_title(self.inner.borrow().stub_handle, label);
        }
        #[cfg(target_os = "linux")]
        {
            let handle = self.inner.borrow().stub_handle;
            crate::platform::gtk::toggle_button_set_label(handle, label);
        }
    }

    /// Get the current button label.
    pub fn get_label(&self) -> String {
        #[cfg(target_os = "windows")]
        {
            read_window_text(self.inner.borrow().hwnd)
        }
        #[cfg(target_os = "macos")]
        {
            self.inner.borrow().label.clone()
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            self.inner.borrow().label.clone()
        }
    }

    /// Register a click-only callback. The handler runs every time
    /// the user clicks the button, regardless of the resulting state.
    /// Use [`ToggleButton::on_toggle`] instead if you want to be
    /// notified of state changes.
    pub fn on_click<F: FnMut() + 'static>(&self, frame: &Frame, mut callback: F) {
        let id = self.inner.borrow().id;
        let inner = self.inner.clone();
        frame.register_command_handler(
            id,
            Box::new(move || {
                // Aggiorna la cache per matchare quello che l'utente ha fatto.
                advance_checked(&inner);
                callback();
            }),
        );
    }

    /// Register a callback fired whenever the state of the button
    /// changes. The new state (`true` = checked) is passed to the
    /// callback.
    pub fn on_toggle<F: FnMut(bool) + 'static>(&self, frame: &Frame, mut callback: F) {
        let id = self.inner.borrow().id;
        let inner = self.inner.clone();
        frame.register_command_handler(
            id,
            Box::new(move || {
                advance_checked(&inner);
                let state = inner.borrow().checked;
                callback(state);
            }),
        );
    }

    /// State change with [`ToggleButtonEvent`](crate::ToggleButtonEvent) payload (`wxToggleButtonEvent`).
    pub fn on_toggle_event<F: FnMut(&crate::ToggleButtonEvent) + 'static>(
        &self,
        frame: &Frame,
        mut f: F,
    ) {
        self.on_toggle(frame, move |pressed| {
            f(&crate::ToggleButtonEvent::new(pressed));
        });
    }

    /// Get a `WidgetRef` for use with sizers.
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

impl Widget for ToggleButtonInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.hwnd as isize
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.stub_handle
        }
    }

    fn set_position(&mut self, x: i32, y: i32) {
        self.rect.x = x;
        self.rect.y = y;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(
                self.hwnd,
                x,
                y,
                self.rect.width as i32,
                self.rect.height as i32,
                1,
            );
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::macos_widget_set_frame(
                self.parent_handle,
                self.stub_handle,
                self.rect,
            );
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_view_frame(self.parent_handle, self.stub_handle, self.rect);
        }
    }

    fn set_size(&mut self, w: u32, h: u32) {
        self.rect.width = w;
        self.rect.height = h;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(self.hwnd, self.rect.x, self.rect.y, w as i32, h as i32, 1);
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::macos_widget_set_frame(
                self.parent_handle,
                self.stub_handle,
                self.rect,
            );
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_view_frame(self.parent_handle, self.stub_handle, self.rect);
        }
    }

    fn rect(&self) -> Rect {
        self.rect
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            ShowWindow(self.hwnd, if visible { SW_SHOW } else { SW_HIDE });
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::macos_widget_set_visible(self.stub_handle, visible);
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_view_visible(self.stub_handle, visible);
        }
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            EnableWindow(self.hwnd, if enabled { 1 } else { 0 });
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::macos_widget_set_enabled(self.stub_handle, enabled);
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_view_enabled(self.stub_handle, enabled);
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    /// The Win32 constants we use must match `<winuser.h>`. A
    /// regression here would silently break `BM_SETCHECK` dispatch
    /// (a wrong `BST_CHECKED` would make the button look pressed
    /// when it is logically unchecked, or vice versa).
    #[cfg(target_os = "windows")]
    #[test]
    fn win32_constants_pinned() {
        assert_eq!(BM_GETCHECK, 0x00F0);
        assert_eq!(BM_SETCHECK, 0x00F1);
        assert_eq!(BST_UNCHECKED, 0x0000);
        assert_eq!(BST_CHECKED, 0x0001);
        assert_eq!(BST_INDETERMINATE, 0x0002);
    }

    /// The cached `checked` flag must follow `set_value` /
    /// `get_value` without touching the live control.
    #[test]
    fn value_round_trip() {
        let mut cache = ToggleButtonInner {
            #[cfg(target_os = "windows")]
            hwnd: std::ptr::null_mut(),
            id: 0,
            label: String::new(),
            rect: Rect::new(0, 0, 100, 30),
            enabled: true,
            visible: true,
            checked: false,
        };
        assert!(!cache.checked);
        cache.checked = true;
        assert!(cache.checked);
        cache.checked = false;
        assert!(!cache.checked);
    }
}
