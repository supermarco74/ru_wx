//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Simple drop-down list (`wxChoice`).
//!
//! On Windows this is a Win32 combo box created with the
//! `CBS_DROPDOWNLIST` style: a non-editable text field plus a drop-down
//! list. The user picks an item from the list; the text field is
//! always read-only.
//!
//! Selection changes are reported via `WM_COMMAND` with notification
//! code `CBN_SELCHANGE`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::window::frame::Frame;
use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};

use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── Win32 ComboBox constants ─────────────────────────────────────────

#[cfg(target_os = "windows")]
const CB_ADDSTRING: u32 = 0x0143;
#[cfg(target_os = "windows")]
const CB_INSERTSTRING: u32 = 0x014A;
#[cfg(target_os = "windows")]
const CB_DELETESTRING: u32 = 0x0144;
#[cfg(target_os = "windows")]
const CB_RESETCONTENT: u32 = 0x014B;
#[cfg(target_os = "windows")]
const CB_SETCURSEL: u32 = 0x014E;
#[cfg(target_os = "windows")]
const CB_GETCURSEL: u32 = 0x0147;
#[cfg(target_os = "windows")]
const CB_GETCOUNT: u32 = 0x0146;
#[cfg(target_os = "windows")]
const CB_GETLBTEXT: u32 = 0x0148;
#[cfg(target_os = "windows")]
const CB_GETLBTEXTLEN: u32 = 0x0149;
#[cfg(target_os = "windows")]
const CB_ERR: isize = -1;

/// ComboBox style: drop-down list (no edit field, just a static
/// selection box and a drop-down).
#[cfg(target_os = "windows")]
const CBS_DROPDOWNLIST: u32 = 0x0003;
// We use the more specific CBS_DROPDOWNLIST | CBS_HASSTRINGS to ensure
// the control allocates string storage and reports CBN_SELCHANGE
// (rather than the owner-draw CBN_SELENDOK that DROPDOWNLIST alone
// uses in some configurations).

/// ComboBox style: owner-draw disabled.
#[cfg(target_os = "windows")]
const CBS_HASSTRINGS: u32 = 0x0200;

// ── Inner type ───────────────────────────────────────────────────────

struct ChoiceInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    rect: Rect,
    enabled: bool,
    visible: bool,
}

#[derive(Clone)]
pub struct Choice {
    inner: Rc<RefCell<ChoiceInner>>,
}

impl Choice {
    /// Create a new choice control as a child of the given parent window.
    pub fn new<W: Window>(parent_in: &W) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = parent_in.hwnd();
            let wide_class = to_wide("COMBOBOX");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | CBS_DROPDOWNLIST | CBS_HASSTRINGS,
                0,
                0,
                200,
                28, // visible drop-down height (not the pop-down list)
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = parent_in;

        Choice {
            inner: Rc::new(RefCell::new(ChoiceInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                rect: Rect::new(0, 0, 200, 28),
                enabled: true,
                visible: true,
            })),
        }
    }

    /// Append an item to the drop-down list.
    pub fn append(&self, item: &str) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(item);
            SendMessageW(
                self.inner.borrow().hwnd,
                CB_ADDSTRING,
                0,
                wide.as_ptr() as isize,
            );
        }
    }

    /// Insert an item at the given zero-based index.
    pub fn insert(&self, index: usize, item: &str) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(item);
            SendMessageW(
                self.inner.borrow().hwnd,
                CB_INSERTSTRING,
                index,
                wide.as_ptr() as isize,
            );
        }
    }

    /// Remove the item at the given zero-based index.
    pub fn remove(&self, index: usize) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, CB_DELETESTRING, index, 0);
        }
    }

    /// Remove all items.
    pub fn clear(&self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, CB_RESETCONTENT, 0, 0);
        }
    }

    /// Return the number of items.
    pub fn get_count(&self) -> usize {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: FFI call to SendMessageW; `hwnd` is a live window and `msg` / `wParam` / `lParam` are valid for that window.
            let result = unsafe { SendMessageW(self.inner.borrow().hwnd, CB_GETCOUNT, 0, 0) };
            // `CB_GETCOUNT` returns `CB_ERR` (-1) on failure; the
            // unchecked cast would turn it into `usize::MAX`.
            if result < 0 {
                0
            } else {
                result as usize
            }
        }
        #[cfg(not(target_os = "windows"))]
        0
    }

    /// Return the index of the currently selected item, or `None` if
    /// no item is selected.
    pub fn get_selection(&self) -> Option<usize> {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let result = unsafe { SendMessageW(self.inner.borrow().hwnd, CB_GETCURSEL, 0, 0) };
            if result == CB_ERR {
                None
            } else {
                Some(result as usize)
            }
        }
        #[cfg(not(target_os = "windows"))]
        None
    }

    /// Select the item at the given index.
    pub fn set_selection(&self, index: usize) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, CB_SETCURSEL, index, 0);
        }
    }

    /// Return the text of the item at the given index, or `None` if
    /// the index is out of range.
    pub fn get_string(&self, index: usize) -> Option<String> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            // SAFETY: FFI call to SendMessageW; `hwnd` is a live window and `msg` / `wParam` / `lParam` are valid for that window.
            let len = unsafe { SendMessageW(hwnd, CB_GETLBTEXTLEN, index, 0) };
            if len == CB_ERR {
                return None;
            }
            let mut buf = vec![0u16; (len as usize) + 1];
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let result =
                unsafe { SendMessageW(hwnd, CB_GETLBTEXT, index, buf.as_mut_ptr() as isize) };
            if result == CB_ERR {
                return None;
            }
            Some(String::from_utf16_lossy(&buf[..len as usize]))
        }
        #[cfg(not(target_os = "windows"))]
        None
    }

    /// Register a callback fired when the selection changes.
    pub fn on_selection_change<F: FnMut() + 'static>(&self, frame: &Frame, callback: F) {
        let id = self.inner.borrow().id;
        // CBN_SELCHANGE is delivered via WM_COMMAND with id = control
        // id; the frame's dispatcher will route it correctly.
        frame.register_command_handler(id, Box::new(callback));
    }

    /// Selection with [`ChoiceEvent`] payload (`wxChoiceEvent`).
    pub fn on_choice_event<F: FnMut(&crate::ChoiceEvent) + 'static>(
        &self,
        frame: &Frame,
        mut f: F,
    ) {
        let ctrl = self.clone();
        self.on_selection_change(frame, move || {
            let sel = ctrl.get_selection().unwrap_or(0);
            f(&crate::ChoiceEvent::new(sel));
        });
    }

    /// The control's id (used internally for WM_COMMAND dispatch).
    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    /// Get a `WidgetRef` for use with sizers.
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

impl Widget for ChoiceInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.hwnd as isize
        }
        #[cfg(not(target_os = "windows"))]
        0
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
    }

    fn set_size(&mut self, w: u32, h: u32) {
        self.rect.width = w;
        self.rect.height = h;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(self.hwnd, self.rect.x, self.rect.y, w as i32, h as i32, 1);
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
    }
}

impl crate::core::item_container::ItemContainer for Choice {
    fn count(&self) -> usize {
        self.get_count()
    }

    fn get_string(&self, index: usize) -> Option<String> {
        Choice::get_string(self, index)
    }

    fn append(&self, item: &str) {
        Choice::append(self, item);
    }

    fn clear(&self) {
        Choice::clear(self);
    }
}

impl crate::core::item_container_immutable::ItemContainerImmutable for Choice {
    fn count(&self) -> usize {
        self.get_count()
    }

    fn get_string(&self, index: usize) -> Option<String> {
        Choice::get_string(self, index)
    }
}
