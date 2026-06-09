use std::cell::RefCell;
use std::rc::Rc;

use crate::frame::Frame;
use crate::geometry::Rect;
use crate::widget::{Widget, WidgetRef};

#[cfg(target_os = "windows")]
use crate::platform::win32::{next_control_id, to_wide};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── Win32 TreeView constants ─────────────────────────────────────────

#[cfg(target_os = "windows")]
const TVM_INSERTITEMW: u32 = 0x1132;
#[cfg(target_os = "windows")]
const TVM_DELETEITEM: u32 = 0x1101;
#[cfg(target_os = "windows")]
const TVM_EXPAND: u32 = 0x1102;
#[cfg(target_os = "windows")]
const TVM_GETNEXTITEM: u32 = 0x110A;
#[cfg(target_os = "windows")]
const TVM_SETITEMW: u32 = 0x113F;

/// TVGN_CARET — retrieve the currently selected item
#[cfg(target_os = "windows")]
const TVGN_CARET: u32 = 9;
/// TVE_EXPAND — expand the item
#[cfg(target_os = "windows")]
const TVE_EXPAND: u32 = 2;
/// TVE_COLLAPSE — collapse the item
#[cfg(target_os = "windows")]
const TVE_COLLAPSE: u32 = 1;

/// TVN_SELCHANGED — TreeView notification code, sent after the
/// selection changes (NMHDR.code = 0xFFFFFE6E).
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const TVN_SELCHANGED: u32 = 0xFFFFFE6E;

/// TVI_ROOT — insert as a root item
#[cfg(target_os = "windows")]
const TVI_ROOT: isize = 0xFFFF0000usize as isize;
/// TVI_LAST — insert at the end of the sibling list
#[cfg(target_os = "windows")]
const TVI_LAST: isize = 0xFFFF0002usize as isize;

/// TreeView window styles
#[cfg(target_os = "windows")]
const TVS_HASLINES: u32 = 2;
#[cfg(target_os = "windows")]
const TVS_LINESATROOT: u32 = 4;
#[cfg(target_os = "windows")]
const TVS_HASBUTTONS: u32 = 1;

/// TVITEMW mask flag
#[cfg(target_os = "windows")]
const TVIF_TEXT: u32 = 1;

// ── Win32 structs ────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(clippy::upper_case_acronyms)]
struct TVINSERTSTRUCTW {
    h_parent: isize,
    h_insert_after: isize,
    item: TVITEMW,
}

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(clippy::upper_case_acronyms)]
#[allow(non_snake_case)]
struct TVITEMW {
    mask: u32,
    h_item: isize,
    state: u32,
    state_mask: u32,
    psz_text: *mut u16,
    cch_text_max: i32,
    i_image: i32,
    i_selected_image: i32,
    c_children: i32,
    l_param: isize,
}

// ── TreeItem handle ──────────────────────────────────────────────────

/// A handle to an item in the TreeView (wraps HTREEITEM).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TreeItem(pub isize);

// ── Inner type ───────────────────────────────────────────────────────

struct TreeCtrlInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    rect: Rect,
    enabled: bool,
    visible: bool,
    /// User-supplied on-selection-change callback, if any. Receives the
    /// newly selected `TreeItem` (or `None` if the selection was
    /// cleared). Stored in the inner state so the WM_NOTIFY handler
    /// registered on the parent `Frame` can reach it.
    on_sel_change: Option<Box<dyn FnMut(Option<TreeItem>)>>,
}

// ── Public type ──────────────────────────────────────────────────────

#[derive(Clone)]
pub struct TreeCtrl {
    inner: Rc<RefCell<TreeCtrlInner>>,
}

impl TreeCtrl {
    /// Create a new TreeView control as a child of the given frame.
    pub fn new(frame: &Frame) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = frame.hwnd();
            let wide_class = to_wide("SysTreeView32");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | WS_BORDER | TVS_HASLINES | TVS_LINESATROOT | TVS_HASBUTTONS,
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

        TreeCtrl {
            inner: Rc::new(RefCell::new(TreeCtrlInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                rect: Rect::new(0, 0, 200, 200),
                enabled: true,
                visible: true,
                on_sel_change: None,
            })),
        }
    }

    /// Add a root-level item. Returns a `TreeItem` handle for the new item.
    pub fn add_root(&self, text: &str) -> TreeItem {
        #[cfg(target_os = "windows")]
        {
            let wide = to_wide(text);
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let mut insert: TVINSERTSTRUCTW = unsafe { std::mem::zeroed() };
            insert.h_parent = TVI_ROOT;
            insert.h_insert_after = TVI_LAST;
            insert.item.mask = TVIF_TEXT;
            insert.item.psz_text = wide.as_ptr() as *mut u16;
            insert.item.cch_text_max = 0;

            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let result = unsafe {
                SendMessageW(
                    self.inner.borrow().hwnd,
                    TVM_INSERTITEMW,
                    0,
                    &insert as *const TVINSERTSTRUCTW as isize,
                )
            };
            TreeItem(result)
        }

        #[cfg(not(target_os = "windows"))]
        TreeItem(0)
    }

    /// Append a child item under the given parent. Returns a `TreeItem` handle.
    pub fn append_item(&self, parent: TreeItem, text: &str) -> TreeItem {
        #[cfg(target_os = "windows")]
        {
            let wide = to_wide(text);
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let mut insert: TVINSERTSTRUCTW = unsafe { std::mem::zeroed() };
            insert.h_parent = parent.0;
            insert.h_insert_after = TVI_LAST;
            insert.item.mask = TVIF_TEXT;
            insert.item.psz_text = wide.as_ptr() as *mut u16;
            insert.item.cch_text_max = 0;

            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let result = unsafe {
                SendMessageW(
                    self.inner.borrow().hwnd,
                    TVM_INSERTITEMW,
                    0,
                    &insert as *const TVINSERTSTRUCTW as isize,
                )
            };
            TreeItem(result)
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = (parent, text);
            TreeItem(0)
        }
    }

    /// Delete the given item and all its children.
    pub fn delete_item(&self, item: TreeItem) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, TVM_DELETEITEM, 0, item.0);
        }
    }

    /// Delete all items in the tree view.
    pub fn delete_all_items(&self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(self.inner.borrow().hwnd, TVM_DELETEITEM, 0, TVI_ROOT);
        }
    }

    /// Return the currently selected item, or `None` if nothing is selected.
    pub fn get_selection(&self) -> Option<TreeItem> {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let result = unsafe {
                SendMessageW(
                    self.inner.borrow().hwnd,
                    TVM_GETNEXTITEM,
                    TVGN_CARET as usize,
                    0,
                )
            };
            if result != 0 {
                Some(TreeItem(result))
            } else {
                None
            }
        }

        #[cfg(not(target_os = "windows"))]
        None
    }

    /// Change the text of an existing tree item.
    pub fn set_item_text(&self, item: TreeItem, text: &str) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let wide = to_wide(text);
            let mut tvitem: TVITEMW = std::mem::zeroed();
            tvitem.mask = TVIF_TEXT;
            tvitem.h_item = item.0;
            tvitem.psz_text = wide.as_ptr() as *mut u16;
            tvitem.cch_text_max = 0;

            SendMessageW(
                self.inner.borrow().hwnd,
                TVM_SETITEMW,
                0,
                &tvitem as *const TVITEMW as isize,
            );
        }
    }

    /// Expand the given item to show its children.
    pub fn expand(&self, item: TreeItem) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(
                self.inner.borrow().hwnd,
                TVM_EXPAND,
                TVE_EXPAND as usize,
                item.0,
            );
        }
    }

    /// Collapse the given item to hide its children.
    pub fn collapse(&self, item: TreeItem) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(
                self.inner.borrow().hwnd,
                TVM_EXPAND,
                TVE_COLLAPSE as usize,
                item.0,
            );
        }
    }

    /// Register a callback that fires when the user selects a different
    /// tree item. The callback receives the newly selected `TreeItem`,
    /// or `None` if the selection is cleared.
    ///
    /// The TreeView notifies its parent via `WM_NOTIFY` (not
    /// `WM_COMMAND`), so this method registers a `WM_NOTIFY` handler on
    /// the supplied `Frame`. The handler filters for the
    /// `TVN_SELCHANGED` notification code, then queries the TreeView
    /// for the current selection with `TVM_GETNEXTITEM` / `TVGN_CARET`
    /// and passes it to the user callback.
    pub fn on_selection_change<F: FnMut(Option<TreeItem>) + 'static>(
        &self,
        frame: &Frame,
        callback: F,
    ) {
        // Store the user's callback inside our inner state.
        self.inner.borrow_mut().on_sel_change = Some(Box::new(callback));

        // Register a WM_NOTIFY handler on the frame that:
        //   1. Filters for the TVN_SELCHANGED notification code.
        //   2. Queries the TreeView for the current selection.
        //   3. Fires the user's callback with that selection.
        let inner = self.inner.clone();
        let id = self.inner.borrow().id;
        frame.register_notify_handler(
            id,
            Box::new(move |code| {
                if code != TVN_SELCHANGED {
                    return;
                }
                #[cfg(target_os = "windows")]
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                unsafe {
                    let new_selection = {
                        let hwnd = inner.borrow().hwnd;
                        let r = SendMessageW(hwnd, TVM_GETNEXTITEM, TVGN_CARET as usize, 0);
                        if r != 0 {
                            Some(TreeItem(r))
                        } else {
                            None
                        }
                    };

                    // Fire the user's callback. Take it out, call it,
                    // put it back — the same pattern the frame uses
                    // for its command/notify dispatchers.
                    let cb = inner.borrow_mut().on_sel_change.take();
                    if let Some(mut c) = cb {
                        c(new_selection);
                        inner.borrow_mut().on_sel_change = Some(c);
                    }
                }

                #[cfg(not(target_os = "windows"))]
                {
                    let _ = (inner, code);
                }
            }),
        );
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

// ── Widget trait ─────────────────────────────────────────────────────

impl Widget for TreeCtrlInner {
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
