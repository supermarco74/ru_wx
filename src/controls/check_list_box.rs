//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! wxCheckListBox — a `ListBox` with a per-item check-box.
//!
//! On Windows there is no `LBS_CHECKBOXES` style, so we use a regular
//! `LISTBOX` window and keep the per-item checked state in this struct.
//! Clicking an item fires an `LBN_SELCHANGE` notification; we auto-toggle
//! the stored check state and invoke the registered callback with the
//! index that was toggled and its new state.
//!
//! Use [`CheckListBox::new`] to create an empty list, then call
//! [`CheckListBox::append`] / [`CheckListBox::insert`] to populate it.
//! Use [`CheckListBox::check`] / [`CheckListBox::is_checked`] to read or
//! write a per-item checked state, and [`CheckListBox::on_check_toggle`]
//! to be notified when the user toggles a check-box.

use std::cell::RefCell;
use std::rc::Rc;

use crate::window::frame::Frame;
use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};

#[cfg(target_os = "windows")]
use crate::platform::win32::{next_control_id, to_wide};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── Win32 ListBox constants ────────────────────────────────────────────

#[cfg(target_os = "windows")]
const LB_ADDSTRING: u32 = 0x0180;
#[cfg(target_os = "windows")]
const LB_INSERTSTRING: u32 = 0x0181;
#[cfg(target_os = "windows")]
const LB_DELETESTRING: u32 = 0x0182;
#[cfg(target_os = "windows")]
const LB_RESETCONTENT: u32 = 0x0184;
#[cfg(target_os = "windows")]
const LB_SETCURSEL: u32 = 0x0186;
#[cfg(target_os = "windows")]
const LB_GETCURSEL: u32 = 0x0188;
#[cfg(target_os = "windows")]
#[allow(dead_code)] // Win32 ABI surface — used by future get-text/count helpers
const LB_GETTEXT: u32 = 0x0189;
#[cfg(target_os = "windows")]
#[allow(dead_code)] // Win32 ABI surface — used by future get-text/count helpers
const LB_GETTEXTLEN: u32 = 0x018A;
#[cfg(target_os = "windows")]
#[allow(dead_code)] // Win32 ABI surface — used by future get-text/count helpers
const LB_GETCOUNT: u32 = 0x018B;
#[cfg(target_os = "windows")]
const LB_GETSELCOUNT: u32 = 0x0190;
#[cfg(target_os = "windows")]
const LB_GETSELITEMS: u32 = 0x0191;
#[cfg(target_os = "windows")]
const LB_ERR: isize = -1;

#[cfg(target_os = "windows")]
const LBS_NOTIFY: u32 = 0x0001;

// ── Inner type ─────────────────────────────────────────────────────────

struct CheckListBoxInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    rect: Rect,
    /// Mirror of the listbox content (kept here so we can map the
    /// currently-selected row to a checked flag without re-querying the
    /// listbox for text on every toggle).
    items: Vec<String>,
    /// Parallel array of checked states; `checked[i]` is the check state
    /// of `items[i]`.
    checked: Vec<bool>,
    enabled: bool,
    visible: bool,
}

// ── Public type ────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct CheckListBox {
    inner: Rc<RefCell<CheckListBoxInner>>,
}

impl CheckListBox {
    /// Create a new check-listbox as a child of the given parent window.
    pub fn new<W: Window>(parent_in: &W) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = parent_in.hwnd();
            let wide_class = to_wide("LISTBOX");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | WS_BORDER | WS_VSCROLL | LBS_NOTIFY,
                0,
                0,
                200,
                200,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = parent_in;

        CheckListBox {
            inner: Rc::new(RefCell::new(CheckListBoxInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                rect: Rect::new(0, 0, 200, 200),
                items: Vec::new(),
                checked: Vec::new(),
                enabled: true,
                visible: true,
            })),
        }
    }

    /// Append an item (unchecked by default) to the end of the list.
    pub fn append(&self, item: &str) {
        let mut state = self.inner.borrow_mut();
        state.items.push(item.to_string());
        state.checked.push(false);
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(item);
            SendMessageW(state.hwnd, LB_ADDSTRING, 0, wide.as_ptr() as isize);
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = item;
        }
    }

    /// Insert an item at the given zero-based index.
    pub fn insert(&self, index: usize, item: &str) {
        let mut state = self.inner.borrow_mut();
        let clamped = index.min(state.items.len());
        state.items.insert(clamped, item.to_string());
        state.checked.insert(clamped, false);
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(item);
            SendMessageW(state.hwnd, LB_INSERTSTRING, clamped, wide.as_ptr() as isize);
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (index, item);
        }
    }

    /// Remove the item at the given zero-based index.
    pub fn remove(&self, index: usize) {
        let mut state = self.inner.borrow_mut();
        if index < state.items.len() {
            state.items.remove(index);
            state.checked.remove(index);
        }
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(state.hwnd, LB_DELETESTRING, index, 0);
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = index;
        }
    }

    /// Remove all items from the list.
    pub fn clear(&self) {
        self.inner.borrow_mut().items.clear();
        self.inner.borrow_mut().checked.clear();
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, LB_RESETCONTENT, 0, 0);
        }
    }

    /// Return the total number of items in the list.
    pub fn get_count(&self) -> usize {
        self.inner.borrow().items.len()
    }

    /// Return the index of the currently highlighted item, or `None`.
    pub fn get_selection(&self) -> Option<usize> {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let result = unsafe { SendMessageW(self.inner.borrow().hwnd, LB_GETCURSEL, 0, 0) };
            if result == LB_ERR {
                None
            } else {
                Some(result as usize)
            }
        }
        #[cfg(not(target_os = "windows"))]
        None
    }

    /// Return the indices of all selected (highlighted) items.
    pub fn get_selections(&self) -> Vec<usize> {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let count = unsafe { SendMessageW(self.inner.borrow().hwnd, LB_GETSELCOUNT, 0, 0) };
            if count <= 0 {
                return Vec::new();
            }
            let mut buf = vec![0u32; count as usize];
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let filled = unsafe {
                SendMessageW(
                    self.inner.borrow().hwnd,
                    LB_GETSELITEMS,
                    count as usize,
                    buf.as_mut_ptr() as isize,
                )
            };
            if filled <= 0 {
                return Vec::new();
            }
            buf[..filled as usize].iter().map(|&i| i as usize).collect()
        }
        #[cfg(not(target_os = "windows"))]
        Vec::new()
    }

    /// Highlight the item at the given index.
    pub fn set_selection(&self, index: usize) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, LB_SETCURSEL, index, 0);
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = index;
        }
    }

    /// Return the text of the item at the given index, or `None`.
    pub fn get_string(&self, index: usize) -> Option<String> {
        self.inner.borrow().items.get(index).cloned()
    }

    /// Set the checked state of the item at the given index.
    pub fn check(&self, index: usize, checked: bool) {
        let mut state = self.inner.borrow_mut();
        if let Some(flag) = state.checked.get_mut(index) {
            *flag = checked;
        }
    }

    /// Return the checked state of the item at the given index.
    pub fn is_checked(&self, index: usize) -> bool {
        self.inner
            .borrow()
            .checked
            .get(index)
            .copied()
            .unwrap_or(false)
    }

    /// Return a `Vec<bool>` of the checked state of every item in the list.
    pub fn get_checked_items(&self) -> Vec<bool> {
        self.inner.borrow().checked.clone()
    }

    /// Register a callback that fires when the user clicks an item
    /// (the underlying listbox fires `LBN_SELCHANGE`).
    ///
    /// The struct auto-toggles the stored check state for the clicked
    /// item and the callback receives the index and the new checked
    /// state. After the callback returns, the highlighted item is
    /// deselected so the row does not appear "stuck".
    pub fn on_check_toggle<F: FnMut(usize, bool) + 'static>(&self, frame: &Frame, mut callback: F) {
        let id = self.inner.borrow().id;
        let inner = self.inner.clone();
        frame.register_command_handler(
            id,
            Box::new(move || {
                #[cfg(target_os = "windows")]
                {
                    let hwnd = inner.borrow().hwnd;
                    // SAFETY: FFI call to SendMessageW; `hwnd` is a live window and `msg` / `wParam` / `lParam` are valid for that window.
                    let sel = unsafe { SendMessageW(hwnd, LB_GETCURSEL, 0, 0) };
                    if sel < 0 {
                        return;
                    }
                    let idx = sel as usize;
                    let mut state = inner.borrow_mut();
                    if idx >= state.checked.len() {
                        return;
                    }
                    state.checked[idx] = !state.checked[idx];
                    let new_state = state.checked[idx];
                    // Deselect so the row does not appear "stuck" highlighted
                    // after a click.
                    // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                    unsafe {
                        SendMessageW(state.hwnd, LB_SETCURSEL, usize::MAX, 0);
                    }
                    drop(state);
                    callback(idx, new_state);
                }
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = &inner;
                }
            }),
        );
    }

    /// Get the control ID.
    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    /// Get a `WidgetRef` for use with sizers.
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

// ── Widget trait ───────────────────────────────────────────────────────

impl Widget for CheckListBoxInner {
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
