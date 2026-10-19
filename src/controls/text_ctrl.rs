//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Single- or multi-line text editor (`wxTextCtrl`).
//!
//! On Windows the widget is a standard `EDIT` control. [`TextCtrl::new`]
//! takes a parent and an initial string; [`TextCtrl::set_value`] /
//! [`TextCtrl::get_value`] read and write the live buffer. Multi-line
//! mode is selected with the `multi_line` constructor flag.

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};
use crate::window::frame::Frame;

use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Win32 Edit control styles
#[cfg(target_os = "windows")]
const ES_MULTILINE: u32 = 0x0004;
#[cfg(target_os = "windows")]
const ES_PASSWORD: u32 = 0x0020;
#[cfg(target_os = "windows")]
const ES_AUTOHSCROLL: u32 = 0x0080;
#[cfg(target_os = "windows")]
const ES_AUTOVSCROLL: u32 = 0x0040;
#[cfg(target_os = "windows")]
const ES_WANTRETURN: u32 = 0x1000;
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const ES_READONLY: u32 = 0x0800;

/// Win32 Edit control messages
#[cfg(target_os = "windows")]
const EM_SETREADONLY: u32 = 0x00CF;
#[cfg(target_os = "windows")]
const EM_SETLIMITTEXT: u32 = 0x00C5;
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const EM_GETLIMITTEXT: u32 = 0x00D5;
#[cfg(target_os = "windows")]
const EM_SETSEL: u32 = 0x00B1;
#[cfg(target_os = "windows")]
const EM_REPLACESEL: u32 = 0x00C2;
#[cfg(target_os = "windows")]
const EM_CANUNDO: u32 = 0x00C6;
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const WM_CLEAR: u32 = 0x0303;
#[cfg(target_os = "windows")]
const WM_UNDO: u32 = 0x0304;

/// Win32 Edit control notification code
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const EN_CHANGE: u32 = 0x0300;

struct TextCtrlInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(all(
        not(target_os = "windows"),
        not(target_os = "macos"),
        not(target_os = "linux")
    ))]
    stub_handle: isize,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    stub_handle: isize,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    parent_handle: crate::platform::NativeHandle,
    #[cfg(all(
        not(target_os = "windows"),
        not(target_os = "macos"),
        not(target_os = "linux")
    ))]
    text: String,
    id: u16,
    rect: Rect,
    #[allow(dead_code)]
    multiline: bool,
    enabled: bool,
    visible: bool,
    /// Cached read-only state. We track this in addition to querying the
    /// control so the getter is cheap and we don't need a separate
    /// `is_readonly()` round-trip to Win32.
    readonly: bool,
    /// Cached max-length (characters). 0 means "unlimited" (the Win32
    /// default). Tracked so [`TextCtrl::max_length`] doesn't need a
    /// `WM_GETTEXTLIMIT` round-trip.
    max_length: u32,
}

#[derive(Clone)]
pub struct TextCtrl {
    inner: Rc<RefCell<TextCtrlInner>>,
}

/// Convert bare `\n` line separators into `\r\n` so a Win32 multiline
/// `EDIT` control breaks lines correctly. Existing `\r\n` pairs are
/// left intact (we never produce `\r\r\n`).
///
/// This is needed because the Win32 `EDIT` control treats only the
/// CRLF pair as a hard line break; a lone LF is rendered as an
/// unprintable glyph (or, on most fonts, simply ignored, which makes
/// successive lines run together visually). Cross-platform GUI
/// toolkits (wxWidgets, GTK, Qt) all normalize newlines for the user;
/// we do the same here so callers can keep using ordinary `\n`.
#[cfg(target_os = "windows")]
fn normalize_newlines_for_edit(s: &str) -> String {
    if !s.contains('\n') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len() + 8);
    let mut prev = '\0';
    for c in s.chars() {
        if c == '\n' && prev != '\r' {
            out.push('\r');
        }
        out.push(c);
        prev = c;
    }
    out
}

/// Inverse of [`normalize_newlines_for_edit`]: strip the `\r` from
/// every `\r\n` pair so callers see plain `\n` line separators in
/// the value returned from a multiline control. Lone `\r` characters
/// (rare, but legal) are preserved.
#[cfg(target_os = "windows")]
fn strip_crlf_for_caller(s: &str) -> String {
    if !s.contains('\r') {
        return s.to_string();
    }
    s.replace("\r\n", "\n")
}

impl TextCtrl {
    /// Create a new single-line text input control
    pub fn new<W: Window>(parent_in: &W, default_text: &str) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = crate::platform::window_handle_as_hwnd(parent_in);
            let wide_text = to_wide(default_text);
            let wide_class = to_wide("EDIT");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                wide_text.as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_BORDER | ES_AUTOHSCROLL,
                0,
                0,
                150,
                24,
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
        let _ = parent_in;

        #[cfg(target_os = "macos")]
        let parent_handle = crate::platform::window_handle(parent_in);
        #[cfg(target_os = "macos")]
        let stub_handle = crate::platform::appkit::create_text_field_notify(
            parent_handle,
            default_text,
            Rect::new(0, 0, 150, 24),
            id,
        );

        // GTK4 (Linux): crea la GtkEntry reale (headless: handle stub).
        #[cfg(target_os = "linux")]
        let parent_handle = crate::platform::window_handle(parent_in);
        #[cfg(target_os = "linux")]
        let stub_handle = crate::platform::gtk::create_text_field(
            parent_handle,
            default_text,
            Rect::new(0, 0, 150, 24),
            id,
        );

        TextCtrl {
            inner: Rc::new(RefCell::new(TextCtrlInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(all(
                    not(target_os = "windows"),
                    not(target_os = "macos"),
                    not(target_os = "linux")
                ))]
                stub_handle: crate::platform::stub_backend::alloc_widget_handle(),
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                stub_handle,
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                parent_handle,
                #[cfg(all(
                    not(target_os = "windows"),
                    not(target_os = "macos"),
                    not(target_os = "linux")
                ))]
                text: default_text.to_string(),
                id,
                rect: Rect::new(0, 0, 150, 24),
                multiline: false,
                enabled: true,
                visible: true,
                readonly: false,
                max_length: 0,
            })),
        }
    }

    /// Create a new multi-line text input control
    pub fn multiline<W: Window>(parent_in: &W, default_text: &str) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = crate::platform::window_handle_as_hwnd(parent_in);
            // Create the control with an empty initial title. The
            // multiline EDIT control does not always parse `\r\n`
            // line separators when supplied through `lpWindowName`
            // at creation time — the seeded text would be rendered
            // as a single concatenated visual line. Setting the text
            // via `SetWindowTextW` after creation is the
            // documented-reliable path.
            let wide_class = to_wide("EDIT");
            let empty: [u16; 1] = [0];
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                empty.as_ptr(),
                // NOTE: do NOT include `ES_AUTOHSCROLL` here. In a
                // multiline EDIT control its presence disables word
                // wrapping (the control would scroll horizontally
                // instead of breaking long lines). `ES_WANTRETURN`
                // makes the Enter key insert a newline rather than
                // activating the parent dialog's default button.
                WS_CHILD
                    | WS_VISIBLE
                    | WS_BORDER
                    | ES_MULTILINE
                    | ES_AUTOVSCROLL
                    | ES_WANTRETURN
                    | WS_VSCROLL,
                0,
                0,
                200,
                100,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(target_os = "windows")]
        // Now seed the actual text post-creation, with `\n` -> `\r\n`
        // normalization. SAFETY: hwnd is the just-created control;
        // the wide buffer is null-terminated and lives until end of
        // call.
        unsafe {
            if !default_text.is_empty() {
                let normalized = normalize_newlines_for_edit(default_text);
                let wide = to_wide(&normalized);
                SetWindowTextW(hwnd, wide.as_ptr());
            }
        }

        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        let _ = parent_in;

        #[cfg(target_os = "macos")]
        let parent_handle = crate::platform::window_handle(parent_in);
        #[cfg(target_os = "macos")]
        let stub_handle = crate::platform::appkit::create_multiline_text_ctrl_notify(
            parent_handle,
            default_text,
            Rect::new(0, 0, 200, 100),
            id,
        );

        // GTK4 (Linux): crea la GtkTextView multi-linea reale.
        #[cfg(target_os = "linux")]
        let parent_handle = crate::platform::window_handle(parent_in);
        #[cfg(target_os = "linux")]
        let stub_handle = crate::platform::gtk::create_multiline_text_ctrl(
            parent_handle,
            default_text,
            Rect::new(0, 0, 200, 100),
            id,
        );

        TextCtrl {
            inner: Rc::new(RefCell::new(TextCtrlInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(all(
                    not(target_os = "windows"),
                    not(target_os = "macos"),
                    not(target_os = "linux")
                ))]
                stub_handle: crate::platform::stub_backend::alloc_widget_handle(),
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                stub_handle,
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                parent_handle,
                #[cfg(all(
                    not(target_os = "windows"),
                    not(target_os = "macos"),
                    not(target_os = "linux")
                ))]
                text: default_text.to_string(),
                id,
                rect: Rect::new(0, 0, 200, 100),
                multiline: true,
                enabled: true,
                visible: true,
                readonly: false,
                max_length: 0,
            })),
        }
    }

    /// Create a password text input control (single-line with masked characters)
    pub fn password<W: Window>(parent_in: &W, default_text: &str) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = crate::platform::window_handle_as_hwnd(parent_in);
            let wide_text = to_wide(default_text);
            let wide_class = to_wide("EDIT");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                wide_text.as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_BORDER | ES_PASSWORD | ES_AUTOHSCROLL,
                0,
                0,
                150,
                24,
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
        let _ = parent_in;

        #[cfg(target_os = "macos")]
        let parent_handle = crate::platform::window_handle(parent_in);
        #[cfg(target_os = "macos")]
        let stub_handle = crate::platform::appkit::create_secure_text_field_notify(
            parent_handle,
            default_text,
            Rect::new(0, 0, 150, 24),
            id,
        );

        // GTK4 (Linux): GtkEntry con visibility=false (maschera password).
        #[cfg(target_os = "linux")]
        let parent_handle = crate::platform::window_handle(parent_in);
        #[cfg(target_os = "linux")]
        let stub_handle = crate::platform::gtk::create_secure_text_field(
            parent_handle,
            default_text,
            Rect::new(0, 0, 150, 24),
            id,
        );

        TextCtrl {
            inner: Rc::new(RefCell::new(TextCtrlInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(all(
                    not(target_os = "windows"),
                    not(target_os = "macos"),
                    not(target_os = "linux")
                ))]
                stub_handle: crate::platform::stub_backend::alloc_widget_handle(),
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                stub_handle,
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                parent_handle,
                #[cfg(all(
                    not(target_os = "windows"),
                    not(target_os = "macos"),
                    not(target_os = "linux")
                ))]
                text: default_text.to_string(),
                id,
                rect: Rect::new(0, 0, 150, 24),
                multiline: false,
                enabled: true,
                visible: true,
                readonly: false,
                max_length: 0,
            })),
        }
    }

    /// Get the current text value
    pub fn get_value(&self) -> String {
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            // SAFETY: FFI call to GetWindowTextLengthW; `hwnd` is a real window handle and the wide buffer is sized appropriately.
            //
            // `GetWindowTextLengthW` returns -1 if the window
            // has no title bar / text (e.g. disabled / owner-
            // drawn), so we guard with `<= 0` rather than `== 0`
            // — previously an `-1` return would have been cast
            // to `usize` (producing `usize::MAX`) and triggered
            // a multi-GiB allocation that aborted the process.
            let len = unsafe { GetWindowTextLengthW(hwnd) };
            if len <= 0 {
                return String::new();
            }
            // `len` is i32, so `len as usize` is at most
            // `i32::MAX as usize` (~2 Gi UTF-16 units) which
            // fits in `usize` even on 32-bit hosts. We
            // saturating-add 1 to keep the NUL slot in scope
            // even for the largest legitimate `len`.
            let buf_len = (len as usize).saturating_add(1);
            let mut buf = Vec::with_capacity(buf_len);
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1);
                buf.set_len(buf_len);
            }
            // Convert from UTF-16 to String, stripping the trailing null
            let raw = String::from_utf16_lossy(&buf[..len as usize]);
            // For multiline controls the Win32 `EDIT` stores `\r\n`
            // line breaks; collapse them back to plain `\n` so callers
            // see the same separators they passed in.
            if self.inner.borrow().multiline {
                strip_crlf_for_caller(&raw)
            } else {
                raw
            }
        }

        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        {
            self.inner.borrow().text.clone()
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::get_text_field_value(self.inner.borrow().stub_handle)
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::get_widget_text(self.inner.borrow().stub_handle)
                .unwrap_or_default()
        }
    }

    /// Alias getter sintetico per il testo (pendant di `set_text`).
    /// Delega a [`Self::get_value`], unificando i getter come da
    /// `PIANO_DEBUG_INSPECTOR.md` §A2.
    pub fn value(&self) -> String {
        self.get_value()
    }

    /// Setter sintetico per il testo (pendant di [`Self::value`]).
    /// Delega a [`Self::set_value`].
    pub fn set_text(&self, text: &str) {
        self.set_value(text);
    }

    /// Set the text value
    pub fn set_value(&self, text: &str) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            // Multiline EDIT controls need CRLF separators; normalize
            // here so callers can pass plain `\n`.
            let normalized = if self.inner.borrow().multiline {
                normalize_newlines_for_edit(text)
            } else {
                text.to_string()
            };
            let wide = to_wide(&normalized);
            SetWindowTextW(self.inner.borrow().hwnd, wide.as_ptr());
        }
        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        {
            if self.inner.borrow().readonly {
                return;
            }
            self.inner.borrow_mut().text = text.to_string();
        }
        #[cfg(target_os = "macos")]
        {
            if self.inner.borrow().readonly {
                return;
            }
            crate::platform::appkit::set_text_field_value(self.inner.borrow().stub_handle, text);
        }
        #[cfg(target_os = "linux")]
        {
            if self.inner.borrow().readonly {
                return;
            }
            crate::platform::gtk::set_widget_text(self.inner.borrow().stub_handle, text);
        }
    }

    /// Set or clear read-only mode
    pub fn set_readonly(&self, readonly: bool) {
        self.inner.borrow_mut().readonly = readonly;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(
                self.inner.borrow().hwnd,
                EM_SETREADONLY,
                if readonly { 1 } else { 0 },
                0,
            );
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::text_ctrl_set_readonly(
                self.inner.borrow().stub_handle,
                readonly,
            );
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_widget_readonly(self.inner.borrow().stub_handle, readonly);
        }
    }

    /// `true` if the control is currently in read-only mode.
    pub fn is_readonly(&self) -> bool {
        self.inner.borrow().readonly
    }

    /// Set the maximum number of characters the control will accept.
    ///
    /// Pass `0` to remove the limit (the Win32 default). Setting this
    /// smaller than the current text length truncates the user's
    /// ability to type more, but does not erase existing content.
    pub fn set_max_length(&self, max: u32) {
        self.inner.borrow_mut().max_length = max;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, EM_SETLIMITTEXT, max as usize, 0);
        }
        #[cfg(target_os = "macos")]
        {
            // NSTextField on macOS doesn't have a built-in character limit.
            // The limit is stored but not enforced without a delegate.
            let _ = max;
        }
        #[cfg(target_os = "linux")]
        {
            // GtkEntry supporta il limite di caratteri in modo nativo.
            crate::platform::gtk::set_widget_max_length(
                self.inner.borrow().stub_handle,
                max as i32,
            );
        }
    }

    /// The current maximum number of characters (`0` = unlimited).
    pub fn max_length(&self) -> u32 {
        self.inner.borrow().max_length
    }

    /// Clear the contents of the control.
    pub fn clear(&self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SetWindowTextW(self.inner.borrow().hwnd, std::ptr::null());
        }
        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        {
            self.inner.borrow_mut().text.clear();
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::set_text_field_value(self.inner.borrow().stub_handle, "");
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_widget_text(self.inner.borrow().stub_handle, "");
        }
    }

    /// Append `text` to the end of the control, preserving the
    /// current selection. The cursor / caret is moved to the end of
    /// the inserted text.
    pub fn append_text(&self, text: &str) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            // Move caret to the end. Win32's `EM_SETSEL` interprets
            // `-1` as "end of current text" for both the start and end
            // position, which collapses the selection to the caret at
            // the end of the buffer. The signature in `windows-sys` 0.59
            // is `(HWND, u32, wparam: usize, lparam: isize)` so we cast
            // the start position to `usize` and leave the end as `isize`.
            SendMessageW(hwnd, EM_SETSEL, -1isize as usize, -1isize);
            // Multiline EDIT controls need CRLF separators; normalize
            // here so callers can pass plain `\n`.
            let normalized = if self.inner.borrow().multiline {
                normalize_newlines_for_edit(text)
            } else {
                text.to_string()
            };
            let wide = to_wide(&normalized);
            // SAFETY: lparam is a valid pointer to a UTF-16 buffer; Win32 takes
            // ownership of the buffer only for the duration of the call.
            SendMessageW(hwnd, EM_REPLACESEL, 0, wide.as_ptr() as isize);
        }
        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        {
            if !self.inner.borrow().readonly {
                self.inner.borrow_mut().text.push_str(text);
            }
        }
        #[cfg(target_os = "macos")]
        {
            if !self.inner.borrow().readonly {
                let handle = self.inner.borrow().stub_handle;
                let mut current = crate::platform::appkit::get_text_field_value(handle);
                current.push_str(text);
                crate::platform::appkit::set_text_field_value(handle, &current);
            }
        }
        #[cfg(target_os = "linux")]
        {
            if !self.inner.borrow().readonly {
                let handle = self.inner.borrow().stub_handle;
                let mut current = crate::platform::gtk::get_widget_text(handle).unwrap_or_default();
                current.push_str(text);
                crate::platform::gtk::set_widget_text(handle, &current);
            }
        }
    }

    /// `true` if there are operations in the control's undo stack.
    pub fn can_undo(&self) -> bool {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, EM_CANUNDO, 0, 0) != 0
        }
        #[cfg(target_os = "macos")]
        {
            // NSTextField supports undo natively via undoManager.
            // This would need access to the underlying NSTextField to query undoManager.
            false
        }
        #[cfg(not(target_os = "windows"))]
        #[cfg(not(target_os = "macos"))]
        {
            let _ = self;
            false
        }
    }

    /// Undo the last edit operation in the control, if any.
    pub fn undo(&self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, WM_UNDO, 0, 0);
        }
        #[cfg(target_os = "macos")]
        {
            // NSTextField supports undo natively via undoManager.
            // This would need access to the underlying NSTextField.
        }
        #[cfg(not(target_os = "windows"))]
        #[cfg(not(target_os = "macos"))]
        {
            let _ = self;
        }
    }

    /// Register an on-change callback (fires when the text content changes)
    ///
    /// Registers on the `EN_CHANGE` notification code specifically.
    /// A plain id-only `command_handlers` entry would fire for *every*
    /// EDIT notification — `EN_UPDATE` (0x0400) *and* `EN_CHANGE`
    /// (0x0300) per keystroke, plus `EN_SETFOCUS` / `EN_KILLFOCUS` —
    /// invoking the callback twice per key and on every focus change.
    pub fn on_change<F: FnMut() + 'static>(&self, frame: &Frame, callback: F) {
        let id = self.inner.borrow().id;
        #[cfg(target_os = "windows")]
        {
            const EN_CHANGE: u16 = 0x0300;
            frame.register_command_notify_handler(id, EN_CHANGE, Box::new(callback));
        }
        #[cfg(not(target_os = "windows"))]
        {
            frame.register_command_handler(id, Box::new(callback));
        }
    }

    /// Text change with [`TextEvent`](crate::TextEvent) payload (`wxTextEvent`).
    pub fn on_text_event<F: FnMut(&crate::TextEvent) + 'static>(&self, frame: &Frame, mut f: F) {
        let ctrl = self.clone();
        self.on_change(frame, move || {
            f(&crate::TextEvent::new(ctrl.get_value()));
        });
    }

    /// Handler per il tasto **Invio** (`VK_RETURN`, codice 0x0D) sul
    /// `TextCtrl`. Utile per le barre editabili (es. address bar)
    /// che navigano su Enter.
    ///
    /// Implementato come `WM_COMMAND` con code notifica `EN_RETURN`
    /// (0x0B), generato dal `EDIT`/`COMBOBOX` nativo di Win32 quando
    /// l'utente preme Invio in un campo single-line. Solo Windows:
    /// sulle altre piattaforme è un no-op (gli equivalenti macOS /
    /// Linux si appoggiano su NSTextField/NSTextFieldDelegate e
    /// gtk::Entry, entrambi non coperti da questa firma minima).
    #[cfg(target_os = "windows")]
    pub fn on_enter<F: FnMut() + 'static>(&self, frame: &Frame, callback: F) {
        let id = self.inner.borrow().id;
        const EN_RETURN: u16 = 0x0B;
        frame.register_command_notify_handler(id, EN_RETURN, Box::new(callback));
    }

    #[cfg(not(target_os = "windows"))]
    pub fn on_enter<F: FnMut() + 'static>(&self, _frame: &Frame, _callback: F) {
        // No-op su macOS/Linux: per ora la pressione di Invio su un
        // TextCtrl editabile non naviga. Vedi note su
        // `register_command_notify_handler`.
    }

    /// Get the control ID
    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    /// Get a WidgetRef for use with sizers
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

impl Widget for TextCtrlInner {
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
            crate::platform::appkit::set_view_frame(
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
            crate::platform::appkit::set_view_frame(
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
            crate::platform::appkit::set_view_visible(self.stub_handle, visible);
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
            crate::platform::appkit::set_view_enabled(self.stub_handle, enabled);
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_view_enabled(self.stub_handle, enabled);
        }
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    /// A2: `get_value`/`set_value` passano dal testo nativo
    /// dell'EDIT (senza inspector).
    #[test]
    fn text_ctrl_value_roundtrip_reads_native_text() {
        let frame = Frame::builder()
            .with_title("text-getter")
            .with_size(280, 140)
            .build();
        let txt = TextCtrl::new(&frame, "inizio");
        assert_eq!(txt.get_value(), "inizio");
        txt.set_value("aggiornato");
        assert_eq!(txt.get_value(), "aggiornato");
    }
}
