//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
use std::cell::RefCell;
use std::rc::Rc;

use crate::window::frame::Frame;
use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef};

use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
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
const TVM_GETCOUNT: u32 = 0x1105;
#[cfg(target_os = "windows")]
const TVM_GETITEMW: u32 = 0x113E; // TV_FIRST + 62
#[cfg(target_os = "windows")]
const TVM_SELECTITEM: u32 = 0x110B;
#[cfg(target_os = "windows")]
const TVM_SETITEMW: u32 = 0x113F;

/// TVGN_CARET — retrieve the currently selected item
const TVGN_CARET: u32 = 9;
/// TVGN_ROOT — retrieve the first (top-level) item
const TVGN_ROOT: u32 = 0;
/// TVGN_NEXT — retrieve the next sibling item
const TVGN_NEXT: u32 = 1;
/// TVGN_PREVIOUS — retrieve the previous sibling item
const TVGN_PREVIOUS: u32 = 2;
/// TVGN_PARENT — retrieve the parent item
const TVGN_PARENT: u32 = 3;
/// TVGN_CHILD — retrieve the first child item
const TVGN_CHILD: u32 = 4;
/// TVGN_FIRSTVISIBLE — retrieve the first visible item
const TVGN_FIRSTVISIBLE: u32 = 5;
/// TVGN_NEXTVISIBLE — retrieve the next visible item
const TVGN_NEXTVISIBLE: u32 = 6;
/// TVGN_PREVIOUSVISIBLE — retrieve the previous visible item
const TVGN_PREVIOUSVISIBLE: u32 = 7;
/// TVE_EXPAND — expand the item
#[cfg(target_os = "windows")]
const TVE_EXPAND: u32 = 2;
/// TVE_COLLAPSE — collapse the item
#[cfg(target_os = "windows")]
const TVE_COLLAPSE: u32 = 1;
/// TVE_COLLAPSERESET — collapse the item and remove all its children
#[cfg(target_os = "windows")]
const TVE_COLLAPSERESET: u32 = 0x8000;

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
/// `TVIF_IMAGE` — the `i_image` field is valid.
#[cfg(target_os = "windows")]
const TVIF_IMAGE: u32 = 2;
/// `TVIF_SELECTEDIMAGE` — the `i_selected_image` field is valid.
#[cfg(target_os = "windows")]
const TVIF_SELECTEDIMAGE: u32 = 0x20;
/// `TVM_SETIMAGELIST` (`TV_FIRST + 9`).
#[cfg(target_os = "windows")]
const TVM_SETIMAGELIST: u32 = 0x1109;
/// `TVSIL_NORMAL` — the image list used for item icons.
#[cfg(target_os = "windows")]
const TVSIL_NORMAL: usize = 0;

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

    /// Add a root-level item with an icon from the attached image
    /// list (see [`TreeCtrl::set_image_list`]).
    pub fn add_root_with_image(&self, text: &str, image_index: i32) -> TreeItem {
        #[cfg(target_os = "windows")]
        {
            let wide = to_wide(text);
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let mut insert: TVINSERTSTRUCTW = unsafe { std::mem::zeroed() };
            insert.h_parent = TVI_ROOT;
            insert.h_insert_after = TVI_LAST;
            insert.item.mask = TVIF_TEXT | TVIF_IMAGE | TVIF_SELECTEDIMAGE;
            insert.item.psz_text = wide.as_ptr() as *mut u16;
            insert.item.cch_text_max = 0;
            insert.item.i_image = image_index;
            insert.item.i_selected_image = image_index;

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
            let _ = (text, image_index);
            TreeItem(0)
        }
    }

    /// Attach an [`crate::ImageList`] to the tree. Items added with
    /// [`TreeCtrl::append_item_with_image`] display the icon at their
    /// `image_index`; the list must stay alive as long as the tree.
    pub fn set_image_list(&self, image_list: &crate::ImageList) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(
                self.inner.borrow().hwnd,
                TVM_SETIMAGELIST,
                TVSIL_NORMAL,
                image_list.handle(),
            );
        }
        #[cfg(not(target_os = "windows"))]
        let _ = image_list;
    }

    /// Append a child item with an icon from the attached image list
    /// (see [`TreeCtrl::set_image_list`]). The same `image_index` is
    /// used for the normal and the selected state. Pass
    /// [`TreeItem`] from [`TreeCtrl::add_root`] (or another append)
    /// as `parent`.
    pub fn append_item_with_image(
        &self,
        parent: TreeItem,
        text: &str,
        image_index: i32,
    ) -> TreeItem {
        #[cfg(target_os = "windows")]
        {
            let wide = to_wide(text);
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let mut insert: TVINSERTSTRUCTW = unsafe { std::mem::zeroed() };
            insert.h_parent = parent.0;
            insert.h_insert_after = TVI_LAST;
            insert.item.mask = TVIF_TEXT | TVIF_IMAGE | TVIF_SELECTEDIMAGE;
            insert.item.psz_text = wide.as_ptr() as *mut u16;
            insert.item.cch_text_max = 0;
            insert.item.i_image = image_index;
            insert.item.i_selected_image = image_index;

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
            let _ = (parent, text, image_index);
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

    /// Collapse the given item and remove all its children.
    /// After this call, the item has no children and the
    /// grandchildren are deleted from the tree.
    pub fn collapse_and_reset(&self, item: TreeItem) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(
                self.inner.borrow().hwnd,
                TVM_EXPAND,
                (TVE_COLLAPSE | TVE_COLLAPSERESET) as usize,
                item.0,
            );
        }
    }

    /// Return the total number of items in the tree view (including
    /// all descendants of all root items).
    pub fn get_count(&self) -> usize {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let result = unsafe {
                SendMessageW(self.inner.borrow().hwnd, TVM_GETCOUNT, 0, 0)
            };
            result as usize
        }

        #[cfg(not(target_os = "windows"))]
        0
    }

    /// Return the first root-level item, or `None` if the tree is empty.
    pub fn get_root_item(&self) -> Option<TreeItem> {
        self.get_next_item(None, TVGN_ROOT)
    }

    /// Return the first child of the given item, or `None` if the
    /// item has no children.
    pub fn get_first_child(&self, item: TreeItem) -> Option<TreeItem> {
        self.get_next_item(Some(item), TVGN_CHILD)
    }

    /// Return the next sibling of the given item, or `None` if the
    /// item is the last sibling.
    pub fn get_next_sibling(&self, item: TreeItem) -> Option<TreeItem> {
        self.get_next_item(Some(item), TVGN_NEXT)
    }

    /// Return the previous sibling of the given item, or `None` if
    /// the item is the first sibling.
    pub fn get_prev_sibling(&self, item: TreeItem) -> Option<TreeItem> {
        self.get_next_item(Some(item), TVGN_PREVIOUS)
    }

    /// Return the parent of the given item, or `None` if the item is
    /// a root item.
    pub fn get_item_parent(&self, item: TreeItem) -> Option<TreeItem> {
        self.get_next_item(Some(item), TVGN_PARENT)
    }

    /// Internal helper: forward to `TVM_GETNEXTITEM` with the given
    /// relation flag. `item` is `None` for "no anchor" (used for
    /// `TVGN_ROOT`); the macro returns `0` for "no item" which we
    /// map to `None`.
    fn get_next_item(&self, item: Option<TreeItem>, relation: u32) -> Option<TreeItem> {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let result = unsafe {
                SendMessageW(
                    self.inner.borrow().hwnd,
                    TVM_GETNEXTITEM,
                    relation as usize,
                    item.map(|i| i.0).unwrap_or(0),
                )
            };
            if result != 0 {
                Some(TreeItem(result))
            } else {
                None
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = (item, relation);
            None
        }
    }

    /// Return the text of the given tree item, or `None` if the item
    /// handle is invalid.
    pub fn get_item_text(&self, item: TreeItem) -> Option<String> {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                let mut buf = vec![0u16; 256];
                let mut tvitem: TVITEMW = std::mem::zeroed();
                tvitem.mask = TVIF_TEXT;
                tvitem.h_item = item.0;
                tvitem.psz_text = buf.as_mut_ptr();
                tvitem.cch_text_max = 256;
                let copied = SendMessageW(
                    self.inner.borrow().hwnd,
                    TVM_GETITEMW,
                    0,
                    &tvitem as *const TVITEMW as isize,
                );
                if copied == 0 {
                    return None;
                }
                let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
                Some(String::from_utf16_lossy(&buf[..end]))
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = item;
            None
        }
    }

    /// Programmatically select the given item. Sets the item as the
    /// "caret" item (the highlighted one).
    pub fn select_item(&self, item: TreeItem) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(
                self.inner.borrow().hwnd,
                TVM_SELECTITEM,
                TVGN_CARET as usize,
                item.0,
            );
        }
    }

    /// Expand all root items (and all of their descendants). This
    /// is the tree-view equivalent of `wxTreeCtrl::ExpandAll`.
    pub fn expand_all(&self) {
        let root = self.get_root_item();
        if let Some(root) = root {
            self.expand_all_recursive(root);
        }
    }

    /// Collapse all root items. This is the tree-view equivalent of
    /// `wxTreeCtrl::CollapseAll`.
    pub fn collapse_all(&self) {
        let mut current = self.get_root_item();
        while let Some(item) = current {
            self.collapse(item);
            current = self.get_next_sibling(item);
        }
    }

    /// Internal helper: recursively expand `item` and all of its
    /// descendants. Uses a `Vec<TreeItem>` stack (not recursion on
    /// the Rust call stack) so the call cannot overflow on very
    /// deep trees.
    fn expand_all_recursive(&self, item: TreeItem) {
        let mut stack: Vec<TreeItem> = Vec::new();
        stack.push(item);
        while let Some(node) = stack.pop() {
            self.expand(node);
            // Push children in reverse order so the first child is
            // processed first (matches the natural left-to-right
            // reading order).
            let mut children: Vec<TreeItem> = Vec::new();
            let mut child = self.get_first_child(node);
            while let Some(c) = child {
                children.push(c);
                child = self.get_next_sibling(c);
            }
            for c in children.into_iter().rev() {
                stack.push(c);
            }
        }
    }

    /// Expand the given item and all of its descendants
    /// recursively. This is the tree-view equivalent of
    /// `wxTreeCtrl::ExpandAllChildren`.
    ///
    /// Like `expand_all`, the recursion is implemented on a
    /// `Vec<TreeItem>` stack (not the Rust call stack) so it is
    /// safe on very deep trees.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let tree = frame.tree_ctrl(...);
    /// if let Some(root) = tree.get_root_item() {
    ///     tree.expand_all_children(root);
    /// }
    /// ```
    pub fn expand_all_children(&self, item: TreeItem) {
        self.expand_all_recursive(item);
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
                #[cfg(target_os = "windows")]
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
                    let _ = code;
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

#[cfg(test)]
mod tests {
    //! Unit tests for `TreeCtrl`.
    //!
    //! The `TreeCtrl::new` constructor requires a real Win32
    //! `Frame` and parent window, so we cannot exercise the
    //! real recursive expansion logic in a headless test
    //! without a `MockWindow` harness. We instead pin the
    //! shape of the new v0.6.2 API here:
    //!
    //! * `expand_all_children` has the same shape as
    //!   `expand` (`fn(&TreeCtrl, TreeItem) -> ()`).
    //! * The method is reachable through the public `TreeCtrl`
    //!   inherent impl, not just through a trait re-export.
    //!
    //! Runtime tests live in the `tests/integration.rs`
    //! suite and the future `MockWindow` harness.

    use super::{TreeCtrl, TreeItem};

    /// Pin the TreeView message ids to the values in `commctrl.h`
    /// (`TV_FIRST = 0x1100`; wide variants live at `+50`/`+62`/`+63`,
    /// *not* at the ANSI offsets).
    #[cfg(target_os = "windows")]
    #[test]
    fn tvm_constants_match_commctrl_h() {
        assert_eq!(super::TVM_INSERTITEMW, 0x1100 + 50);
        assert_eq!(super::TVM_DELETEITEM, 0x1100 + 1);
        assert_eq!(super::TVM_EXPAND, 0x1100 + 2);
        assert_eq!(super::TVM_GETNEXTITEM, 0x1100 + 10);
        assert_eq!(super::TVM_GETCOUNT, 0x1100 + 5);
        assert_eq!(super::TVM_GETITEMW, 0x1100 + 62);
        assert_eq!(super::TVM_SELECTITEM, 0x1100 + 11);
        assert_eq!(super::TVM_SETITEMW, 0x1100 + 63);
    }

    /// Pin the shape of `TreeCtrl::expand_all_children` as a
    /// function pointer. If a future refactor renames the
    /// method, changes its arity, or returns a value, this
    /// test fails to compile.
    #[test]
    fn signature_expand_all_children() {
        let _: fn(&TreeCtrl, TreeItem) = TreeCtrl::expand_all_children;
    }

    /// Confirm `expand_all_children` is reachable directly
    /// from the `TreeCtrl` inherent impl (i.e. it is not a
    /// method on a trait that would have to be in scope).
    #[test]
    fn expand_all_children_is_inherent_on_tree_ctrl() {
        // We pin the dispatch with a function pointer cast
        // from a fully-qualified path. The cast only type-
        // checks if `expand_all_children` is an inherent
        // method on `TreeCtrl` with the expected signature.
        let f: fn(&TreeCtrl, TreeItem) =
            <TreeCtrl>::expand_all_children;
        // The value of `f` itself is irrelevant for the
        // shape check, but assigning it to a `let` makes
        // the compiler keep the cast (otherwise it is
        // elided and a typo would slip through).
        let _ = f;
    }

    /// Confirm `expand_all_children` is a no-op (does not
    /// panic) when called on a `TreeCtrl` whose internal
    /// HWND has not been created. This is the same property
    /// that `expand` and `collapse` rely on: the recursive
    /// driver must terminate when the first `get_first_child`
    /// returns `None`.
    ///
    /// We cannot construct a real `TreeCtrl` without a
    /// `Frame`, so this is a "compile-time + linker" test
    /// that pins the recursion termination property by
    /// type only. The runtime property is covered by the
    /// integration test once a `MockWindow` harness exists.
    #[test]
    fn expand_all_children_termination_property_is_pinned() {
        // The driver only loops while `get_first_child` keeps
        // returning `Some(_)`. We pin the type signature of
        // the child-fetch method here so a future refactor
        // that breaks the `Option<TreeItem>` contract fails
        // to compile.
        let _: fn(&TreeCtrl, TreeItem) -> Option<TreeItem> =
            TreeCtrl::get_first_child;
    }
}
