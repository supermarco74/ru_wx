//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
use std::cell::RefCell;
use std::rc::Rc;

use crate::window::frame::Frame;
use crate::core::geometry::Rect;
use crate::dc::image_list::ImageList;
use crate::core::widget::{Widget, WidgetRef, Window};

#[cfg(target_os = "windows")]
use crate::platform::win32::{next_control_id, to_wide};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Win32 ComboBox style constants
#[cfg(target_os = "windows")]
const CBS_DROPDOWN: u32 = 0x0002;
#[cfg(target_os = "windows")]
const CBS_DROPDOWNLIST: u32 = 0x0003;

/// Win32 ComboBox messages
#[cfg(target_os = "windows")]
const CB_ADDSTRING: u32 = 0x0143;
#[cfg(target_os = "windows")]
const CB_DELETESTRING: u32 = 0x0144;
#[cfg(target_os = "windows")]
const CB_GETCOUNT: u32 = 0x0146;
#[cfg(target_os = "windows")]
const CB_GETCURSEL: u32 = 0x0147;
#[cfg(target_os = "windows")]
const CB_RESETCONTENT: u32 = 0x014B;
#[cfg(target_os = "windows")]
const CB_INSERTSTRING: u32 = 0x014A;
#[cfg(target_os = "windows")]
const CB_SETCURSEL: u32 = 0x014E;
#[cfg(target_os = "windows")]
const CB_GETLBTEXT: u32 = 0x0148;
#[cfg(target_os = "windows")]
const CB_GETLBTEXTLEN: u32 = 0x0149;
#[cfg(target_os = "windows")]
const CB_ERR: isize = -1;

struct ComboBoxInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    rect: Rect,
    #[allow(dead_code)]
    editable: bool,
    enabled: bool,
    visible: bool,
}

#[derive(Clone)]
pub struct ComboBox {
    inner: Rc<RefCell<ComboBoxInner>>,
}

impl ComboBox {
    /// Create an editable combo box (CBS_DROPDOWN — user can type or select)
    pub fn new<W: Window>(parent_in: &W) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = parent_in.hwnd();
            let wide_class = to_wide("COMBOBOX");
            // Height=200 specifies the drop-down list height, not the control height
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | CBS_DROPDOWN,
                0,
                0,
                150,
                200,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = parent_in;

        ComboBox {
            inner: Rc::new(RefCell::new(ComboBoxInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                rect: Rect::new(0, 0, 150, 24),
                editable: true,
                enabled: true,
                visible: true,
            })),
        }
    }

    /// Create a read-only dropdown choice (CBS_DROPDOWNLIST — user can only select)
    pub fn choice<W: Window>(parent_in: &W) -> Self {
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
                WS_CHILD | WS_VISIBLE | CBS_DROPDOWNLIST,
                0,
                0,
                150,
                200,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = parent_in;

        ComboBox {
            inner: Rc::new(RefCell::new(ComboBoxInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                rect: Rect::new(0, 0, 150, 24),
                editable: false,
                enabled: true,
                visible: true,
            })),
        }
    }

    /// Append an item to the end of the list
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

    /// Insert an item at the specified index
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

    /// Remove the item at the specified index
    pub fn remove(&self, index: usize) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, CB_DELETESTRING, index, 0);
        }
    }

    /// Remove all items from the combo box
    pub fn clear(&self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, CB_RESETCONTENT, 0, 0);
        }
    }

    /// Get the index of the currently selected item, or None if nothing is selected
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

    /// Set the selected item by index
    pub fn set_selection(&self, index: usize) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, CB_SETCURSEL, index, 0);
        }
    }

    /// Get the text in the edit field (only meaningful for editable combos)
    pub fn get_value(&self) -> String {
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            // SAFETY: FFI call to GetWindowTextLengthW; `hwnd` is a real window handle and the wide buffer is sized appropriately.
            //
            // `GetWindowTextLengthW` returns -1 if the window
            // has no title bar / text (e.g. a disabled combo),
            // so we guard with `<= 0` to avoid casting a
            // negative `i32` to `usize` (which would have
            // produced `usize::MAX` and a multi-GiB alloc).
            let len = unsafe { GetWindowTextLengthW(hwnd) };
            if len <= 0 {
                return String::new();
            }
            let buf_len = (len as usize).saturating_add(1);
            let mut buf = Vec::with_capacity(buf_len);
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1);
                buf.set_len(buf_len);
            }
            String::from_utf16_lossy(&buf[..len as usize])
        }

        #[cfg(not(target_os = "windows"))]
        String::new()
    }

    /// Set the text in the edit field (only meaningful for editable combos)
    pub fn set_value(&self, text: &str) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(text);
            SetWindowTextW(self.inner.borrow().hwnd, wide.as_ptr());
        }
    }

    /// Return the text of the item at the given index.
    pub fn get_string(&self, index: usize) -> Option<String> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            let len = unsafe { SendMessageW(hwnd, CB_GETLBTEXTLEN, index, 0) };
            // Reject every negative length (`CB_ERR` is -1 but any
            // anomalous negative value would otherwise wrap to a
            // huge allocation), and use `saturating_add` for the
            // NUL slot like `BitmapComboBox::get_string` does.
            if len < 0 {
                return None;
            }
            let mut buf = vec![0u16; (len as usize).saturating_add(1)];
            let result =
                unsafe { SendMessageW(hwnd, CB_GETLBTEXT, index, buf.as_mut_ptr() as isize) };
            if result == CB_ERR {
                return None;
            }
            Some(String::from_utf16_lossy(&buf[..len as usize]))
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = index;
            None
        }
    }

    /// Get the number of items in the list
    pub fn get_count(&self) -> usize {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let result = unsafe { SendMessageW(self.inner.borrow().hwnd, CB_GETCOUNT, 0, 0) };
            if result == CB_ERR {
                0
            } else {
                result as usize
            }
        }

        #[cfg(not(target_os = "windows"))]
        0
    }

    /// Register a callback that fires when the selection changes
    pub fn on_selection_change<F: FnMut() + 'static>(&self, frame: &Frame, callback: F) {
        let id = self.inner.borrow().id;
        frame.register_command_handler(id, Box::new(callback));
    }

    /// Selection with [`ComboBoxEvent`] payload (`wxComboBoxEvent`).
    pub fn on_combo_event<F: FnMut(&crate::ComboBoxEvent) + 'static>(
        &self,
        frame: &Frame,
        mut f: F,
    ) {
        let ctrl = self.clone();
        self.on_selection_change(frame, move || {
            let sel = ctrl.get_selection().unwrap_or(0);
            f(&crate::ComboBoxEvent::new(sel));
        });
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

impl crate::core::item_container::ItemContainer for ComboBox {
    fn count(&self) -> usize {
        self.get_count()
    }

    fn get_string(&self, index: usize) -> Option<String> {
        ComboBox::get_string(self, index)
    }

    fn append(&self, item: &str) {
        ComboBox::append(self, item);
    }

    fn clear(&self) {
        ComboBox::clear(self);
    }
}

impl crate::core::item_container_immutable::ItemContainerImmutable for ComboBox {
    fn count(&self) -> usize {
        self.get_count()
    }

    fn get_string(&self, index: usize) -> Option<String> {
        ComboBox::get_string(self, index)
    }
}

impl Widget for ComboBoxInner {
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
            MoveWindow(self.hwnd, x, y, self.rect.width as i32, 200, 1);
        }
    }

    fn set_size(&mut self, w: u32, h: u32) {
        self.rect.width = w;
        self.rect.height = h;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            // For combobox, the height parameter in MoveWindow specifies the dropdown height,
            // not the visible control height. Use 200 for dropdown height.
            MoveWindow(self.hwnd, self.rect.x, self.rect.y, w as i32, 200, 1);
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

// =============================================================================
// BitmapComboBox — a ComboBox where each row can carry a small icon.
//
// On Windows this is the `WC_COMBOBOXEX` window class (registered as
// `"ComboBoxEx32"` by `comctl32.dll`). It is an extended combo box that
// holds an `HIMAGELIST`; every row is rendered as `[icon | text]`, and the
// edit field can show the icon of the currently selected row.
//
// This is the closest Win32 equivalent of wxWidgets' `wxBitmapComboBox`.
// A true `wxOwnerDrawnComboBox` (where each row is painted by the
// application via `WM_DRAWITEM` / `WM_MEASUREITEM`) is not implemented
// here — it would require owner-draw item plumbing that is a much larger
// surface. For the common "row with a small icon" use case,
// `BitmapComboBox` is sufficient.
// =============================================================================

/// Win32 ComboBoxEx mask / style bits.
///
/// Only the bits used by this module are defined locally — not all are
/// exported by `windows-sys 0.59`.
#[cfg(target_os = "windows")]
mod bitmap_combo_box_win32 {
    pub const CBEIF_TEXT: u32 = 0x0001;
    pub const CBEIF_IMAGE: u32 = 0x0002;
    pub const CBEIF_SELECTEDIMAGE: u32 = 0x0004;
    pub const CBEIF_OVERLAY: u32 = 0x0008;
    pub const CBEIF_INDENT: u32 = 0x0010;
    pub const CBEIF_LPARAM: u32 = 0x0020;

    /// `CBEM_SETIMAGELIST` (`WM_USER + 2`) — attaches an
    /// `HIMAGELIST` to the control.
    pub const CBEM_SETIMAGELIST: u32 = 0x0402;
    /// `CBEM_INSERTITEMW` (`WM_USER + 11`) — inserts a new row with
    /// an image and a text (wide variant, matches `ComboBoxExItemW`).
    pub const CBEM_INSERTITEM: u32 = 0x040B;
    /// `CBEM_GETIMAGELIST` (`WM_USER + 3`) — returns the attached
    /// `HIMAGELIST`.
    pub const CBEM_GETIMAGELIST: u32 = 0x0403;
    /// `CBEM_SETITEMW` (`WM_USER + 12`) — updates the image / text of
    /// an existing row (wide variant, matches `ComboBoxExItemW`).
    pub const CBEM_SETITEM: u32 = 0x040C;
    /// `CBEM_DELETEITEM` — removes a row. The header defines it as an
    /// alias of `CB_DELETESTRING` (the ComboBoxEx forwards it).
    pub const CBEM_DELETEITEM: u32 = 0x0144;
    /// Row count. There is no `CBEM_GETCOUNT` in the headers — the
    /// ComboBoxEx forwards plain `CB_GETCOUNT` to its inner combo.
    pub const CBEM_GETCOUNT: u32 = 0x0146;
    /// Set the selected row. Forwarded `CB_SETCURSEL` (no dedicated
    /// `CBEM_` message exists).
    pub const CBEM_SETCURSEL: u32 = 0x014E;
    /// Return the selected row (or `CB_ERR`). Forwarded
    /// `CB_GETCURSEL`.
    pub const CBEM_GETCURSEL: u32 = 0x0147;
    /// `CBEM_SETEXTENDEDSTYLE` (`WM_USER + 14`) — toggles the
    /// extended styles.
    pub const CBEM_SETEXTENDEDSTYLE: u32 = 0x040E;
    /// `CBES_EX_NOEDITIMAGE` — do not draw the selected icon inside
    /// the edit (read-only text) field. wxWidgets does the same.
    pub const CBES_EX_NOEDITIMAGE: u32 = 0x0000_0001;
}

#[cfg(target_os = "windows")]
use bitmap_combo_box_win32::*;

/// Layout of the native `COMBOBOXEXITEMW` struct.
///
/// `SendMessageW(CBEM_INSERTITEM, ..., &mut item)` copies the struct
/// before returning, so the wide string it points at is only borrowed
/// for the duration of the call.
#[cfg(target_os = "windows")]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct ComboBoxExItemW {
    mask: u32,
    i_item: isize,
    psz_text: *mut u16,
    cch_text_max: i32,
    i_image: i32,
    i_selected_image: i32,
    i_overlay: i32,
    i_indent: i32,
    l_param: isize,
}

struct BitmapComboBoxInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    rect: Rect,
    enabled: bool,
    visible: bool,
}

#[derive(Clone)]
pub struct BitmapComboBox {
    inner: Rc<RefCell<BitmapComboBoxInner>>,
}

impl BitmapComboBox {
    /// Create a new bitmap combo box. The control is a child of
    /// `parent_in` and starts empty (call [`BitmapComboBox::set_image_list`]
    /// to attach icons and [`BitmapComboBox::append_with_image`] /
    /// [`BitmapComboBox::insert_with_image`] to populate it).
    pub fn new<W: Window>(parent_in: &W) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = parent_in.hwnd();
            let wide_class = to_wide("ComboBoxEx32");
            // The "200" height argument is the drop-down list height, not
            // the control's own height (this matches the regular ComboBox
            // behaviour).
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | CBS_DROPDOWN,
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

        let bcb = BitmapComboBox {
            inner: Rc::new(RefCell::new(BitmapComboBoxInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                rect: Rect::new(0, 0, 200, 24),
                enabled: true,
                visible: true,
            })),
        };

        // Don't paint the icon inside the read-only edit field — the
        // dropdown list shows it on every row, and a cramped icon in
        // the edit field just looks broken. wxWidgets sets the same
        // extended style by default.
        //
        // `CBEM_SETEXTENDEDSTYLE`'s `wParam` is a *mask* (which extended
        // style bits to affect) and `lParam` is the new value. Passing
        // `wParam = 0` would mean "affect no bits", so the style would
        // silently stay at zero — the bit we want has to go in `wParam`
        // as well.
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(
                bcb.inner.borrow().hwnd,
                CBEM_SETEXTENDEDSTYLE,
                CBES_EX_NOEDITIMAGE as usize,
                CBES_EX_NOEDITIMAGE as isize,
            );
        }

        bcb
    }

    /// Attach an image list. Rows added with [`BitmapComboBox::append_with_image`]
    /// / [`BitmapComboBox::insert_with_image`] will display the icon at
    /// the supplied `image_index`. Pass `image_index = -1` to skip the
    /// icon for a particular row.
    #[cfg(target_os = "windows")]
    pub fn set_image_list(&self, image_list: &ImageList) {
        let hwnd = self.inner.borrow().hwnd;
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(hwnd, CBEM_SETIMAGELIST, 0, image_list.handle());
        }
    }

    /// Non-Windows stub.
    #[cfg(not(target_os = "windows"))]
    pub fn set_image_list(&self, _image_list: &ImageList) {}

    /// Append a new row at the end of the list, paired with an image
    /// from the attached image list. `image_index` is a zero-based
    /// index into the image list; pass `-1` for a plain text row.
    ///
    /// Returns the new row's index, or `-1` on failure.
    pub fn append_with_image(&self, text: &str, image_index: i32) -> i32 {
        let count = self.get_count();
        self.insert_with_image(count, text, image_index)
    }

    /// Insert a new row at `index`, paired with an image from the
    /// attached image list. `image_index` is a zero-based index into
    /// the image list; pass `-1` for a plain text row.
    ///
    /// Returns the new row's index, or `-1` on failure.
    pub fn insert_with_image(&self, index: usize, text: &str, image_index: i32) -> i32 {
        #[cfg(target_os = "windows")]
        {
            let inner = self.inner.borrow_mut();
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                let mut wide = to_wide(text);
                let mut item = ComboBoxExItemW {
                    mask: CBEIF_TEXT | CBEIF_IMAGE | CBEIF_SELECTEDIMAGE,
                    i_item: index as isize,
                    psz_text: wide.as_mut_ptr(),
                    cch_text_max: wide.len() as i32,
                    i_image: image_index,
                    i_selected_image: image_index,
                    i_overlay: 0,
                    i_indent: 0,
                    l_param: 0,
                };
                let result = SendMessageW(
                    inner.hwnd,
                    CBEM_INSERTITEM,
                    0,
                    &mut item as *mut _ as isize,
                );
                // `wide` is dropped here, *after* SendMessageW has copied
                // the string out of it (CBEM_INSERTITEM is synchronous).
                drop(wide);
                result as i32
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = (index, text, image_index);
            0
        }
    }

    /// Append a plain (no-image) row. Equivalent to
    /// `append_with_image(text, -1)`.
    pub fn append(&self, text: &str) -> i32 {
        self.append_with_image(text, -1)
    }

    /// Remove the row at `index`. Panics-free; indices out of range are
    /// silently ignored by the underlying control.
    pub fn remove(&self, index: usize) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, CBEM_DELETEITEM, index, 0);
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = index;
        }
    }

    /// Remove every row.
    pub fn clear(&self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            // CB_RESETCONTENT is also the message every regular ComboBox
            // uses — it is forwarded by the ComboBoxEx to its inner
            // child combo, so it works here too. We use the raw value
            // (0x014B) because `CB_RESETCONTENT` is already declared in
            // this file but `CBEM_*` masks are not.
            SendMessageW(self.inner.borrow().hwnd, 0x014B, 0, 0);
        }
    }

    /// Number of rows currently in the list.
    pub fn get_count(&self) -> usize {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let result = unsafe { SendMessageW(self.inner.borrow().hwnd, CBEM_GETCOUNT, 0, 0) };
            if result < 0 {
                0
            } else {
                result as usize
            }
        }

        #[cfg(not(target_os = "windows"))]
        0
    }

    /// Index of the currently selected row, or `None` if nothing is
    /// selected.
    pub fn get_selection(&self) -> Option<usize> {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let result = unsafe { SendMessageW(self.inner.borrow().hwnd, CBEM_GETCURSEL, 0, 0) };
            if result < 0 {
                None
            } else {
                Some(result as usize)
            }
        }

        #[cfg(not(target_os = "windows"))]
        None
    }

    /// Set the selected row.
    ///
    /// `CBEM_SETCURSEL` updates the index of the selected row in the
    /// drop-down list, but the ComboBoxEx edit field needs an explicit
    /// `CBEM_SETITEM` (`i_item = -1`) afterwards to display that row's
    /// text. The outer ComboBoxEx HWND is a compound control — its edit
    /// field is a separate child — so a plain `WM_SETTEXT` on the outer
    /// HWND is silently ignored. This matches the wxWidgets behaviour:
    /// `wxBitmapComboBox::SetSelection` routes through the
    /// `wxTextEntry` interface and writes the string into the control.
    pub fn set_selection(&self, index: usize) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            SendMessageW(hwnd, CBEM_SETCURSEL, index, 0);
            // Mirror the selection back into the read-only edit field.
            // If the index is out of range, `get_string` returns "".
            let text = self.get_string(index);
            let mut wide = to_wide(&text);
            let mut item = ComboBoxExItemW {
                mask: CBEIF_TEXT,
                i_item: -1_isize,
                psz_text: wide.as_mut_ptr(),
                cch_text_max: wide.len() as i32,
                i_image: 0,
                i_selected_image: 0,
                i_overlay: 0,
                i_indent: 0,
                l_param: 0,
            };
            SendMessageW(
                hwnd,
                CBEM_SETITEM,
                0,
                &mut item as *mut _ as isize,
            );
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = index;
        }
    }

    /// Text of the row at `index`. Returns an empty string if the
    /// index is out of range.
    pub fn get_string(&self, index: usize) -> String {
        #[cfg(target_os = "windows")]
        {
            // `CB_GETLBTEXTLEN` returns the length *in characters* not
            // including the null terminator, just like `LB_GETTEXTLEN`.
            // A negative return value means the index is invalid (e.g.
            // out of range) — guard with `<= 0` so we never feed a
            // negative `isize` into the `as usize` cast.
            const CB_GETLBTEXTLEN: u32 = 0x0149;
            const CB_GETLBTEXT: u32 = 0x0148;
            let hwnd = self.inner.borrow().hwnd;
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let len = unsafe { SendMessageW(hwnd, CB_GETLBTEXTLEN, index, 0) };
            if len <= 0 {
                return String::new();
            }
            let buf_len = (len as usize).saturating_add(1);
            // Zero-initialised buffer: `CB_GETLBTEXT` expects writable,
            // initialised storage (no `set_len` on uninit memory).
            let mut buf = vec![0u16; buf_len];
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let written = unsafe {
                SendMessageW(hwnd, CB_GETLBTEXT, index, buf.as_mut_ptr() as isize)
            };
            if written <= 0 {
                return String::new();
            }
            let written = (written as usize).min(buf_len);
            String::from_utf16_lossy(&buf[..written])
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = index;
            String::new()
        }
    }

    /// Text in the edit (read-only) field. For a combo with no
    /// selection this is the empty string.
    pub fn get_value(&self) -> String {
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            // SAFETY: FFI call to GetWindowTextLengthW; `hwnd` is a real window handle and the wide buffer is sized appropriately.
            //
            // `GetWindowTextLengthW` returns -1 for a window
            // with no title bar / text; guard with `<= 0` so
            // the cast to `usize` is always non-negative.
            let len = unsafe { GetWindowTextLengthW(hwnd) };
            if len <= 0 {
                return String::new();
            }
            let buf_len = (len as usize).saturating_add(1);
            let mut buf = Vec::with_capacity(buf_len);
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1);
                buf.set_len(buf_len);
            }
            String::from_utf16_lossy(&buf[..len as usize])
        }

        #[cfg(not(target_os = "windows"))]
        String::new()
    }

    /// Set the text in the edit field.
    pub fn set_value(&self, text: &str) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(text);
            SetWindowTextW(self.inner.borrow().hwnd, wide.as_ptr());
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = text;
        }
    }

    /// Register a callback that fires when the selection changes
    /// (`CBN_SELCHANGE`).
    pub fn on_selection_change<F: FnMut() + 'static>(&self, frame: &Frame, callback: F) {
        let id = self.inner.borrow().id;
        frame.register_command_handler(id, Box::new(callback));
    }

    /// Control ID assigned at construction time. Useful for advanced
    /// uses that need to disambiguate `WM_COMMAND` notifications.
    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    /// Get a `WidgetRef` for use with sizers.
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

impl Widget for BitmapComboBoxInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.hwnd as isize
        }
        #[cfg(not(target_os = "windows"))]
        {
            0
        }
    }

    fn set_position(&mut self, x: i32, y: i32) {
        self.rect.x = x;
        self.rect.y = y;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            // Same convention as the regular ComboBox: the height
            // argument to `MoveWindow` is the drop-down list height,
            // not the control's own height.
            MoveWindow(self.hwnd, x, y, self.rect.width as i32, 200, 1);
        }
    }

    fn set_size(&mut self, w: u32, h: u32) {
        self.rect.width = w;
        self.rect.height = h;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(self.hwnd, self.rect.x, self.rect.y, w as i32, 200, 1);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Pin the Win32 constant values so a future `windows-sys` upgrade
    /// does not silently shift the bits we rely on.
    #[cfg(target_os = "windows")]
    #[test]
    fn win32_constants_pinned() {
        // CBEM_* messages (commctrl.h: WM_USER = 0x0400 based, wide
        // variants for the W struct; cursel/count/delete are the
        // forwarded CB_* messages).
        assert_eq!(CBEM_SETIMAGELIST, 0x0402); // WM_USER + 2
        assert_eq!(CBEM_INSERTITEM, 0x040B); // CBEM_INSERTITEMW
        assert_eq!(CBEM_GETIMAGELIST, 0x0403); // WM_USER + 3
        assert_eq!(CBEM_SETITEM, 0x040C); // CBEM_SETITEMW
        assert_eq!(CBEM_DELETEITEM, 0x0144); // CB_DELETESTRING
        assert_eq!(CBEM_GETCURSEL, 0x0147); // CB_GETCURSEL
        assert_eq!(CBEM_GETCOUNT, 0x0146); // CB_GETCOUNT
        assert_eq!(CBEM_SETCURSEL, 0x014E); // CB_SETCURSEL
        assert_eq!(CBEM_SETEXTENDEDSTYLE, 0x040E); // WM_USER + 14

        // CBEIF_* mask bits
        assert_eq!(CBEIF_TEXT, 0x0001);
        assert_eq!(CBEIF_IMAGE, 0x0002);
        assert_eq!(CBEIF_SELECTEDIMAGE, 0x0004);
        assert_eq!(CBEIF_OVERLAY, 0x0008);
        assert_eq!(CBEIF_INDENT, 0x0010);
        assert_eq!(CBEIF_LPARAM, 0x0020);

        // Extended style
        assert_eq!(CBES_EX_NOEDITIMAGE, 0x0000_0001);
    }

    /// Sanity-check the `ComboBoxExItemW` layout. The struct is
    /// `#[repr(C)]` and Win32 calls are made with `&mut item as
    /// *mut _` — if the layout ever changes, the FFI will start
    /// reading garbage. The actual offsets below match `COMBOBOXEXITEMW`
    /// in `<windowsx.h>` / `<commctrl.h>` for a 64-bit process.
    #[cfg(target_os = "windows")]
    #[test]
    fn combo_box_ex_item_w_layout() {
        use std::mem::{offset_of, size_of};
        // On 64-bit Windows the struct is 56 bytes: 4 bytes of padding
        // follow `mask` (so that `i_item` is 8-byte aligned), 4 bytes
        // of padding follow `i_indent` (so that `l_param` is 8-byte
        // aligned), and the trailing size is rounded up to 8 bytes.
        assert_eq!(size_of::<ComboBoxExItemW>(), 56);
        assert_eq!(offset_of!(ComboBoxExItemW, mask), 0);
        assert_eq!(offset_of!(ComboBoxExItemW, i_item), 8);
        assert_eq!(offset_of!(ComboBoxExItemW, psz_text), 16);
        assert_eq!(offset_of!(ComboBoxExItemW, cch_text_max), 24);
        assert_eq!(offset_of!(ComboBoxExItemW, i_image), 28);
        assert_eq!(offset_of!(ComboBoxExItemW, i_selected_image), 32);
        assert_eq!(offset_of!(ComboBoxExItemW, i_overlay), 36);
        assert_eq!(offset_of!(ComboBoxExItemW, i_indent), 40);
        assert_eq!(offset_of!(ComboBoxExItemW, l_param), 48);
    }
}
