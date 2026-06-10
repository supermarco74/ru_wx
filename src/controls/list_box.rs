//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
use std::cell::RefCell;
use std::rc::Rc;

use crate::window::frame::Frame;
use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};

#[cfg(target_os = "windows")]
use crate::platform::win32::{next_control_id, to_wide};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{POINT, *};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── Win32 ListBox constants ──────────────────────────────────────────

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
const LB_GETTEXT: u32 = 0x0189;
#[cfg(target_os = "windows")]
const LB_GETTEXTLEN: u32 = 0x018A;
#[cfg(target_os = "windows")]
const LB_GETCOUNT: u32 = 0x018B;
#[cfg(target_os = "windows")]
const LB_GETSELCOUNT: u32 = 0x0190;
#[cfg(target_os = "windows")]
const LB_GETSELITEMS: u32 = 0x0191;
#[cfg(target_os = "windows")]
const LB_ITEMFROMPOINT: u32 = 0x01A9;

/// ListBox style: send notification messages to parent
#[cfg(target_os = "windows")]
const LBS_NOTIFY: u32 = 1;
/// ListBox style: allow extended multi-selection (Shift+Click, Ctrl+Click)
#[cfg(target_os = "windows")]
const LBS_EXTENDEDSEL: u32 = 0x0800;

/// ListBox notification: selection changed
#[cfg(target_os = "windows")]
#[allow(dead_code)] // Win32 ABI surface — for future event-dispatch wiring
const LBN_SELCHANGE: u32 = 1;
/// ListBox notification: item double-clicked
#[cfg(target_os = "windows")]
#[allow(dead_code)] // Win32 ABI surface — for future event-dispatch wiring
const LBN_DBLCLK: u32 = 2;

// ── Inner type ───────────────────────────────────────────────────────

struct ListBoxInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    rect: Rect,
    multi_select: bool,
    enabled: bool,
    visible: bool,
}

// ── Public type ──────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ListBox {
    inner: Rc<RefCell<ListBoxInner>>,
}

impl ListBox {
    /// Create a new single-selection listbox as a child of the given parent window.
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
                150,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = parent_in;

        ListBox {
            inner: Rc::new(RefCell::new(ListBoxInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                rect: Rect::new(0, 0, 200, 150),
                multi_select: false,
                enabled: true,
                visible: true,
            })),
        }
    }

    /// Create a new multi-selection listbox as a child of the given parent window.
    pub fn multi_select<W: Window>(parent_in: &W) -> Self {
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
                WS_CHILD | WS_VISIBLE | WS_BORDER | WS_VSCROLL | LBS_NOTIFY | LBS_EXTENDEDSEL,
                0,
                0,
                200,
                150,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = parent_in;

        ListBox {
            inner: Rc::new(RefCell::new(ListBoxInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                rect: Rect::new(0, 0, 200, 150),
                multi_select: true,
                enabled: true,
                visible: true,
            })),
        }
    }

    /// Append an item to the end of the listbox.
    pub fn append(&self, item: &str) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(item);
            SendMessageW(
                self.inner.borrow().hwnd,
                LB_ADDSTRING,
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
                LB_INSERTSTRING,
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
            SendMessageW(self.inner.borrow().hwnd, LB_DELETESTRING, index, 0);
        }
    }

    /// Remove all items from the listbox.
    pub fn clear(&self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, LB_RESETCONTENT, 0, 0);
        }
    }

    /// Return the total number of items in the listbox.
    pub fn get_count(&self) -> usize {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let result = unsafe { SendMessageW(self.inner.borrow().hwnd, LB_GETCOUNT, 0, 0) };
            // `LB_GETCOUNT` returns `LB_ERR` (-1) on failure; the
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

    /// Return the index of the currently selected item (single-selection only).
    ///
    /// Returns `None` if no item is selected or the listbox is multi-select.
    pub fn get_selection(&self) -> Option<usize> {
        #[cfg(target_os = "windows")]
        {
            let inner = self.inner.borrow();
            if inner.multi_select {
                return None;
            }
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let result = unsafe { SendMessageW(inner.hwnd, LB_GETCURSEL, 0, 0) };
            if result == LB_ERR as isize {
                None
            } else {
                Some(result as usize)
            }
        }

        #[cfg(not(target_os = "windows"))]
        None
    }

    /// Return the indices of all selected items (multi-selection only).
    ///
    /// Returns an empty vec for single-selection listboxes.
    pub fn get_selections(&self) -> Vec<usize> {
        #[cfg(target_os = "windows")]
        {
            let inner = self.inner.borrow();
            if !inner.multi_select {
                return Vec::new();
            }
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let count = unsafe { SendMessageW(inner.hwnd, LB_GETSELCOUNT, 0, 0) };
            if count <= 0 {
                return Vec::new();
            }
            let mut buf = vec![0u32; count as usize];
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let filled = unsafe {
                SendMessageW(
                    inner.hwnd,
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

    /// Select the item at the given index (single-selection only).
    pub fn set_selection(&self, index: usize) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, LB_SETCURSEL, index, 0);
        }
    }

    /// Return the text of the item at the given index, or `None` if out of range.
    pub fn get_string(&self, index: usize) -> Option<String> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let len = unsafe { SendMessageW(hwnd, LB_GETTEXTLEN, index, 0) };
            if len == LB_ERR as isize || len < 0 {
                return None;
            }
            // `+ 1` is saturating to keep the NUL slot in
            // scope for the largest legitimate length (a
            // listing with `isize::MAX` characters would
            // otherwise wrap when adding the terminator
            // slot). Real list boxes never approach that
            // size — this is purely a defence-in-depth
            // measure against hostile control returns.
            let mut buf = vec![0u16; (len as usize).saturating_add(1)];
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let result =
                unsafe { SendMessageW(hwnd, LB_GETTEXT, index, buf.as_mut_ptr() as isize) };
            if result == LB_ERR as isize {
                return None;
            }
            Some(String::from_utf16_lossy(&buf[..len as usize]))
        }

        #[cfg(not(target_os = "windows"))]
        None
    }

    /// Register a callback that fires when the selection changes (LBN_SELCHANGE).
    pub fn on_selection_change<F: FnMut() + 'static>(&self, frame: &Frame, callback: F) {
        let id = self.inner.borrow().id;
        frame.register_command_notify_handler(id, LBN_SELCHANGE as u16, Box::new(callback));
    }

    /// Register a callback that fires when an item is double-clicked (LBN_DBLCLK).
    pub fn on_double_click<F: FnMut() + 'static>(&self, frame: &Frame, callback: F) {
        let id = self.inner.borrow().id;
        frame.register_command_notify_handler(id, LBN_DBLCLK as u16, Box::new(callback));
    }

    /// Selection with [`ListBoxEvent`] payload (`wxListBoxEvent`).
    pub fn on_listbox_event<F: FnMut(&crate::ListBoxEvent) + 'static>(
        &self,
        frame: &Frame,
        mut f: F,
    ) {
        let ctrl = self.clone();
        self.on_selection_change(frame, move || {
            let sel = ctrl.get_selection().unwrap_or(0);
            f(&crate::ListBoxEvent::new(sel));
        });
    }

    /// Double-click with [`ListBoxEvent`] payload.
    pub fn on_listbox_double_click_event<F: FnMut(&crate::ListBoxEvent) + 'static>(
        &self,
        frame: &Frame,
        mut f: F,
    ) {
        let ctrl = self.clone();
        self.on_double_click(frame, move || {
            let sel = ctrl.get_selection().unwrap_or(0);
            f(&crate::ListBoxEvent::double_click(sel));
        });
    }

    /// Get the control ID
    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    /// Return the native window handle (Windows only).
    #[cfg(target_os = "windows")]
    pub fn hwnd(&self) -> HWND {
        self.inner.borrow().hwnd
    }

    /// Hit-test a screen coordinate and return the list item index under
    /// the cursor, if any.
    #[cfg(target_os = "windows")]
    pub fn item_at_screen_point(&self, screen_x: i32, screen_y: i32) -> Option<usize> {
        let hwnd = self.inner.borrow().hwnd;
        let mut pt = POINT {
            x: screen_x,
            y: screen_y,
        };
        // SAFETY: `hwnd` is a valid list-box handle; `pt` is updated in place.
        if unsafe { ScreenToClient(hwnd, &mut pt) } == 0 {
            return None;
        }
        let lp = ((pt.y as u16 as u32) << 16) | (pt.x as u16 as u32);
        // SAFETY: `LB_ITEMFROMPOINT` expects client coordinates in `lParam`.
        let result = unsafe { SendMessageW(hwnd, LB_ITEMFROMPOINT, 0, lp as isize) };
        if result < 0 {
            return None;
        }
        let outside = ((result >> 16) & 0xFFFF) != 0;
        if outside {
            None
        } else {
            Some((result & 0xFFFF) as usize)
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn item_at_screen_point(&self, _screen_x: i32, _screen_y: i32) -> Option<usize> {
        None
    }

    /// Get a WidgetRef for use with sizers
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

// ── Widget trait ─────────────────────────────────────────────────────

impl Widget for ListBoxInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.hwnd as isize
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

impl crate::core::item_container::ItemContainer for ListBox {
    fn count(&self) -> usize {
        self.get_count()
    }

    fn get_string(&self, index: usize) -> Option<String> {
        self.get_string(index)
    }

    fn append(&self, item: &str) {
        ListBox::append(self, item);
    }

    fn clear(&self) {
        ListBox::clear(self);
    }
}

impl crate::core::item_container_immutable::ItemContainerImmutable for ListBox {
    fn count(&self) -> usize {
        self.get_count()
    }

    fn get_string(&self, index: usize) -> Option<String> {
        self.get_string(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::frame::Frame;

    #[cfg(target_os = "windows")]
    fn null_hwnd_listbox() -> ListBox {
        let frame = Frame::for_testing();
        let lb = ListBox::new(&frame);
        lb.inner.borrow_mut().hwnd = std::ptr::null_mut();
        lb
    }

    #[test]
    fn signature_item_at_screen_point() {
        let _: fn(&ListBox, i32, i32) -> Option<usize> = ListBox::item_at_screen_point;
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn null_hwnd_item_at_screen_point_returns_none() {
        let lb = null_hwnd_listbox();
        assert_eq!(lb.item_at_screen_point(10, 10), None);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn lb_itemfrompoint_constant_is_pinned() {
        assert_eq!(LB_ITEMFROMPOINT, 0x01A9);
    }
}
