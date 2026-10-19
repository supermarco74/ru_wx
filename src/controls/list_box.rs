//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
use std::cell::RefCell;
use std::rc::Rc;

use crate::core::font::Font;
use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};
use crate::window::frame::Frame;

use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
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
/// ListBox notification: item double-clicked (same code as Win32 `LBN_DBLCLK`).
#[cfg(not(target_os = "windows"))]
const LBN_DBLCLK: u16 = 2;

// ── Inner type ───────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn gtk_list_box<W: Window>(
    parent_in: &W,
    id: u16,
    multi_select: bool,
    width: u32,
    height: u32,
) -> (isize, crate::platform::NativeHandle) {
    let parent = crate::platform::window_handle(parent_in);
    let handle = crate::platform::gtk::create_list_box(
        parent,
        multi_select,
        id,
        Rect::new(0, 0, width, height),
    );
    (handle, parent)
}

struct ListBoxInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    stub_handle: isize,
    #[cfg(target_os = "macos")]
    stub_handle: isize,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    parent_handle: crate::platform::NativeHandle,
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
            let parent = crate::platform::window_handle_as_hwnd(parent_in);
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

        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        let _ = parent_in;

        #[cfg(target_os = "macos")]
        let parent_handle = crate::platform::window_handle(parent_in);
        #[cfg(target_os = "macos")]
        let stub_handle = crate::platform::appkit::create_list_box(
            parent_handle,
            id,
            Rect::new(0, 0, 200, 150),
            false,
        );
        #[cfg(target_os = "linux")]
        let (stub_handle, parent_handle) = gtk_list_box(parent_in, id, false, 200, 150);

        ListBox {
            inner: Rc::new(RefCell::new(ListBoxInner {
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
                id,
                rect: Rect::new(0, 0, 200, 150),
                multi_select: false,
                enabled: true,
                visible: true,
            })),
        }
    }

    /// Apply a custom font to the list contents.
    ///
    /// On Windows this sends `WM_SETFONT` with `lParam = 1` so the
    /// control repaints immediately.
    pub fn set_font(&self, font: &Font) {
        #[cfg(target_os = "windows")]
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::WM_SETFONT;
            let hwnd = self.inner.borrow().hwnd;
            windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW(
                hwnd,
                WM_SETFONT,
                font.hfont() as usize,
                1,
            );
        }
        #[cfg(not(target_os = "windows"))]
        let _ = font;
    }

    /// Customise row colours and height (macOS dark-theme friendly).
    pub fn set_appearance(
        &self,
        row_height: f64,
        text: crate::core::geometry::Colour,
        background: crate::core::geometry::Colour,
        alt_background: crate::core::geometry::Colour,
        selection_bg: crate::core::geometry::Colour,
        selection_fg: crate::core::geometry::Colour,
    ) {
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::listbox_set_appearance(
                self.inner.borrow().stub_handle,
                row_height,
                text,
                background,
                alt_background,
                selection_bg,
                selection_fg,
            );
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (
            row_height,
            text,
            background,
            alt_background,
            selection_bg,
            selection_fg,
        );
    }

    /// Create a new multi-selection listbox as a child of the given parent window.
    pub fn multi_select<W: Window>(parent_in: &W) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = crate::platform::window_handle_as_hwnd(parent_in);
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

        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        let _ = parent_in;

        #[cfg(target_os = "macos")]
        let parent_handle = crate::platform::window_handle(parent_in);
        #[cfg(target_os = "macos")]
        let stub_handle = crate::platform::appkit::create_list_box(
            parent_handle,
            id,
            Rect::new(0, 0, 200, 150),
            true,
        );
        #[cfg(target_os = "linux")]
        let (stub_handle, parent_handle) = gtk_list_box(parent_in, id, true, 200, 150);

        ListBox {
            inner: Rc::new(RefCell::new(ListBoxInner {
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

        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::listbox_append_item(self.inner.borrow().stub_handle, item);
        }

        #[cfg(target_os = "linux")]
        {
            // Handle estratto prima della chiamata: GTK può emettere
            // `selected-rows-changed` in modo sincrono (cambio selezione)
            // e un `borrow()` vivo su `inner` causerebbe re-entrancy.
            let handle = self.inner.borrow().stub_handle;
            crate::platform::gtk::list_box_append(handle, item);
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

        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::listbox_insert_item(
                self.inner.borrow().stub_handle,
                index,
                item,
            );
        }

        #[cfg(target_os = "linux")]
        {
            let handle = self.inner.borrow().stub_handle;
            crate::platform::gtk::list_box_insert(handle, index, item);
        }
    }

    /// Remove the item at the given zero-based index.
    pub fn remove(&self, index: usize) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, LB_DELETESTRING, index, 0);
        }

        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::listbox_remove_item(self.inner.borrow().stub_handle, index);
        }

        #[cfg(target_os = "linux")]
        {
            let handle = self.inner.borrow().stub_handle;
            crate::platform::gtk::list_box_remove(handle, index);
        }
    }

    /// Remove all items from the listbox.
    pub fn clear(&self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, LB_RESETCONTENT, 0, 0);
        }

        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::listbox_clear(self.inner.borrow().stub_handle);
        }

        #[cfg(target_os = "linux")]
        {
            let handle = self.inner.borrow().stub_handle;
            crate::platform::gtk::list_box_clear(handle);
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
        {
            #[cfg(target_os = "macos")]
            {
                crate::platform::appkit::listbox_get_count(self.inner.borrow().stub_handle)
            }
            #[cfg(target_os = "linux")]
            {
                crate::platform::gtk::list_box_get_count(self.inner.borrow().stub_handle)
            }
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            {
                0
            }
        }
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
        {
            #[cfg(target_os = "macos")]
            {
                let inner = self.inner.borrow();
                if inner.multi_select {
                    None
                } else {
                    crate::platform::appkit::listbox_get_selection(inner.stub_handle)
                }
            }
            #[cfg(target_os = "linux")]
            {
                let inner = self.inner.borrow();
                if inner.multi_select {
                    None
                } else {
                    crate::platform::gtk::list_box_get_selection(inner.stub_handle)
                }
            }
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            {
                None
            }
        }
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
        {
            #[cfg(target_os = "macos")]
            {
                let inner = self.inner.borrow();
                if inner.multi_select {
                    crate::platform::appkit::listbox_get_selections(inner.stub_handle)
                } else {
                    Vec::new()
                }
            }
            #[cfg(target_os = "linux")]
            {
                let inner = self.inner.borrow();
                if inner.multi_select {
                    crate::platform::gtk::list_box_get_selections(inner.stub_handle)
                } else {
                    Vec::new()
                }
            }
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            {
                Vec::new()
            }
        }
    }

    /// Select the item at the given index (single-selection only).
    pub fn set_selection(&self, index: usize) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, LB_SETCURSEL, index, 0);
        }

        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::listbox_set_selection(self.inner.borrow().stub_handle, index);
        }

        #[cfg(target_os = "linux")]
        {
            // Handle estratto prima della chiamata: GTK emette
            // `selected-rows-changed` in modo sincrono e un `borrow()`
            // ancora vivo su `inner` causerebbe re-entrancy (panic).
            let handle = self.inner.borrow().stub_handle;
            crate::platform::gtk::list_box_set_selection(handle, index);
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
        {
            #[cfg(target_os = "macos")]
            {
                crate::platform::appkit::listbox_get_string(self.inner.borrow().stub_handle, index)
            }
            #[cfg(target_os = "linux")]
            {
                crate::platform::gtk::list_box_get_string(self.inner.borrow().stub_handle, index)
            }
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            {
                None
            }
        }
    }

    /// Register a callback that fires when the selection changes (LBN_SELCHANGE).
    pub fn on_selection_change<F: FnMut() + 'static>(&self, frame: &Frame, callback: F) {
        let id = self.inner.borrow().id;
        #[cfg(target_os = "windows")]
        frame.register_command_notify_handler(id, LBN_SELCHANGE as u16, Box::new(callback));
        #[cfg(target_os = "macos")]
        frame.register_command_handler(id, Box::new(callback));
        #[cfg(target_os = "linux")]
        frame.register_command_handler(id, Box::new(callback));
        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        let _ = (frame, id, callback);
    }

    /// Register a callback that fires when an item is double-clicked (LBN_DBLCLK).
    pub fn on_double_click<F: FnMut() + 'static>(&self, frame: &Frame, callback: F) {
        let id = self.inner.borrow().id;
        #[cfg(target_os = "windows")]
        frame.register_command_notify_handler(id, LBN_DBLCLK as u16, Box::new(callback));
        #[cfg(target_os = "macos")]
        frame.register_command_notify_handler(id, LBN_DBLCLK, Box::new(callback));
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let _ = (frame, id, callback);
    }

    /// Selection with [`ListBoxEvent`](crate::ListBoxEvent) payload (`wxListBoxEvent`).
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

    /// Double-click with [`ListBoxEvent`](crate::ListBoxEvent) payload.
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

#[cfg(all(test, target_os = "windows"))]
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
