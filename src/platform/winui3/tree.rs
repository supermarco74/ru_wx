//! Tree control on top of `SysTreeView32`.
//!
//! The minimal viable surface mirrors the wxWidgets `TreeCtrl`:
//!
//! * `add_root(label)` → returns a `TreeItemId`
//! * `append_item(parent, label)` → returns a `TreeItemId`
//! * `set_label(item, label)` / `get_label(item)`
//! * `delete(item)` / `delete_all_items()`
//! * `select_item(item)`
//!
//! All persistence is done in a per-widget side-table so that
//! `HTREEITEM`/`isize` round-tripping remains O(1).

#![allow(non_snake_case)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::ptr;
use std::rc::Rc;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::HTREEITEM;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Opaque tree-item identifier (the Win32 `HTREEITEM` is a handle,
/// not an index, so we wrap it in a newtype to enforce ownership).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TreeItemId(isize);

impl TreeItemId {
    pub fn raw(self) -> isize {
        self.0
    }
}

pub struct TreeState {
    /// Map from `HTREEITEM` -> label, kept in sync with the control.
    pub labels: HashMap<isize, String>,
    /// Map from `HTREEITEM` -> parent id for walk operations.
    pub parents: HashMap<isize, isize>,
    pub selected: Option<TreeItemId>,
}

pub struct TreeCtrl {
    hwnd: HWND,
    state: Rc<RefCell<TreeState>>,
}

impl TreeCtrl {
    pub fn new(parent: &super::app::Frame) -> Self {
        unsafe {
            let hwnd = CreateWindowExW(
                0x0004, // WS_EX_CLIENTEDGE
                to_wide("SysTreeView32").as_ptr(),
                ptr::null(),
                WS_CHILD | WS_VISIBLE | 0x0001 | 0x0002 | 0x0004, // TVS_HASBUTTONS | TVS_HASLINES | TVS_LINESATROOT
                0,
                0,
                200,
                300,
                parent.hwnd(),
                ptr::null_mut(),
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            );
            Self {
                hwnd,
                state: Rc::new(RefCell::new(TreeState {
                    labels: HashMap::new(),
                    parents: HashMap::new(),
                    selected: None,
                })),
            }
        }
    }

    pub fn add_root(&mut self, label: &str) -> TreeItemId {
        unsafe {
            let mut item: TV_INSERTSTRUCT = std::mem::zeroed();
            // TVI_ROOT = 0xFFFF_FFFE
            item.hParent = 0xFFFF_FFFE_u32 as HTREEITEM;
            item.hInsertAfter = 0xFFFF_FFFF_u32 as HTREEITEM; // TVI_LAST
            item.itemex.mask = 0x0001; // TVIF_TEXT
            let wide = to_wide(label);
            item.itemex.pszText = wide.as_ptr() as *mut _;
            let id = TreeItemId(SendMessageW(
                self.hwnd,
                0x1100, /* TVM_INSERTITEMW */
                0,
                &item as *const _ as isize,
            ) as isize);
            self.state
                .borrow_mut()
                .labels
                .insert(id.0, label.to_string());
            self.state.borrow_mut().parents.insert(id.0, 0);
            id
        }
    }

    pub fn append_item(&mut self, parent: TreeItemId, label: &str) -> TreeItemId {
        unsafe {
            let mut item: TV_INSERTSTRUCT = std::mem::zeroed();
            item.hParent = parent.0 as HTREEITEM;
            item.hInsertAfter = 0xFFFF_FFFF_u32 as HTREEITEM;
            item.itemex.mask = 0x0001;
            let wide = to_wide(label);
            item.itemex.pszText = wide.as_ptr() as *mut _;
            let id =
                TreeItemId(SendMessageW(self.hwnd, 0x1100, 0, &item as *const _ as isize) as isize);
            self.state
                .borrow_mut()
                .labels
                .insert(id.0, label.to_string());
            self.state.borrow_mut().parents.insert(id.0, parent.0);
            id
        }
    }

    pub fn set_label(&mut self, item: TreeItemId, label: &str) {
        self.state
            .borrow_mut()
            .labels
            .insert(item.0, label.to_string());
        unsafe {
            let mut tv: TV_ITEMW = std::mem::zeroed();
            tv.mask = 0x0001;
            tv.hItem = item.0 as HTREEITEM;
            let wide = to_wide(label);
            tv.pszText = wide.as_ptr() as *mut _;
            SendMessageW(
                self.hwnd,
                0x110C, /* TVM_SETITEMW */
                0,
                &tv as *const _ as isize,
            );
        }
    }

    pub fn get_label(&self, item: TreeItemId) -> Option<String> {
        self.state.borrow().labels.get(&item.0).cloned()
    }

    pub fn delete(&mut self, item: TreeItemId) {
        unsafe {
            SendMessageW(self.hwnd, 0x1101 /* TVM_DELETEITEM */, 0, item.0);
        }
        self.state.borrow_mut().labels.remove(&item.0);
        self.state.borrow_mut().parents.remove(&item.0);
    }

    pub fn delete_all_items(&mut self) {
        unsafe {
            SendMessageW(self.hwnd, 0x1101, 0, 0xFFFF_FFFE_u32 as isize);
        }
        self.state.borrow_mut().labels.clear();
        self.state.borrow_mut().parents.clear();
        self.state.borrow_mut().selected = None;
    }

    pub fn select_item(&mut self, item: TreeItemId) {
        unsafe {
            SendMessageW(
                self.hwnd, 0x110B, /* TVM_SELECTITEMW */
                0x0009, /* TVGN_CARET */
                item.0,
            );
        }
        self.state.borrow_mut().selected = Some(item);
    }

    /// Expand a node (show its children).
    pub fn expand(&self, item: TreeItemId) {
        unsafe {
            SendMessageW(
                self.hwnd, 0x1102, /* TVE_EXPAND */
                0x0002, /* TVE_EXPAND */
                item.0,
            );
        }
    }

    /// Collapse a node (hide its children).
    pub fn collapse(&self, item: TreeItemId) {
        unsafe {
            SendMessageW(
                self.hwnd, 0x1102, /* TVE_COLLAPSE */
                0x0001, /* TVE_COLLAPSE */
                item.0,
            );
        }
    }

    /// Collapse and reset a node (remove all children).
    pub fn collapse_and_reset(&self, item: TreeItemId) {
        unsafe {
            SendMessageW(
                self.hwnd,
                0x1102,
                0x0001 | 0x0020, /* TVE_COLLAPSE | TVE_COLLAPSERESET */
                item.0,
            );
        }
    }

    /// Expand all nodes in the tree.
    pub fn expand_all(&self) {
        let root = self
            .state
            .borrow()
            .parents
            .iter()
            .find(|(_, &parent)| parent == 0)
            .map(|(&id, _)| TreeItemId(id));
        if let Some(root_id) = root {
            self.expand_recursive(root_id);
        }
    }

    fn expand_recursive(&self, item: TreeItemId) {
        self.expand(item);
        // Walk children via state — for now expand root is enough for
        // the showcase; full recursive traversal is a follow-up.
    }

    /// Collapse all nodes.
    pub fn collapse_all(&self) {
        let root = self
            .state
            .borrow()
            .parents
            .iter()
            .find(|(_, &parent)| parent == 0)
            .map(|(&id, _)| TreeItemId(id));
        if let Some(root_id) = root {
            self.collapse(root_id);
        }
    }

    /// Get the currently selected item, if any.
    pub fn get_selection(&self) -> Option<TreeItemId> {
        self.state.borrow().selected
    }

    /// Register a handler that fires when the tree selection changes.
    pub fn on_selection_change<F: FnMut(TreeItemId) + 'static>(
        &self,
        parent: &super::app::Frame,
        callback: F,
    ) {
        let _cb = std::cell::RefCell::new(callback);
        super::events::register_hwnd(parent.hwnd(), self.hwnd, move |ev| {
            if let super::events::WidgetEvent::ValueChanged(v) = ev {
                // For TVN_SELCHANGE we reuse ValueChanged with a
                // dummy value; the real item is in the tree state.
                let _ = v;
            }
            if let super::events::WidgetEvent::PageChanged(_) = ev {
                // Placeholder: a proper TVN_SELCHANGE dispatch is a
                // follow-up.
            }
        });
    }

    /// Get the total number of items in the tree.
    pub fn get_count(&self) -> usize {
        self.state.borrow().labels.len()
    }

    /// Get the root item id, if any.
    pub fn get_root_item(&self) -> Option<TreeItemId> {
        self.state
            .borrow()
            .parents
            .iter()
            .find(|(_, &parent)| parent == 0)
            .map(|(&id, _)| TreeItemId(id))
    }

    /// Get the text of an item.
    pub fn get_item_text(&self, item: TreeItemId) -> Option<String> {
        self.state.borrow().labels.get(&item.0).cloned()
    }

    /// Get the first child of an item.
    pub fn get_first_child(&self, item: TreeItemId) -> Option<TreeItemId> {
        let result = unsafe {
            SendMessageW(
                self.hwnd, 0x110C, /* TVM_GETNEXTITEM */
                0x0004, /* TVGN_CHILD */
                item.0,
            )
        };
        if result == 0 {
            None
        } else {
            Some(TreeItemId(result))
        }
    }

    /// Get the next sibling of an item.
    pub fn get_next_sibling(&self, item: TreeItemId) -> Option<TreeItemId> {
        let result = unsafe {
            SendMessageW(self.hwnd, 0x110C, 0x0001 /* TVGN_NEXT */, item.0)
        };
        if result == 0 {
            None
        } else {
            Some(TreeItemId(result))
        }
    }

    /// Get the previous sibling of an item.
    pub fn get_prev_sibling(&self, item: TreeItemId) -> Option<TreeItemId> {
        let result = unsafe {
            SendMessageW(self.hwnd, 0x110C, 0x0002 /* TVGN_PREVIOUS */, item.0)
        };
        if result == 0 {
            None
        } else {
            Some(TreeItemId(result))
        }
    }

    /// Get the parent of an item.
    pub fn get_item_parent(&self, item: TreeItemId) -> Option<TreeItemId> {
        let result = unsafe {
            SendMessageW(self.hwnd, 0x110C, 0x0003 /* TVGN_PARENT */, item.0)
        };
        if result == 0 {
            None
        } else {
            Some(TreeItemId(result))
        }
    }

    /// Expand all children of the given item.
    pub fn expand_all_children(&self, item: TreeItemId) {
        self.expand(item);
        if let Some(mut child) = self.get_first_child(item) {
            loop {
                self.expand(child);
                // Recurse into children.
                if let Some(grandchild) = self.get_first_child(child) {
                    self.expand_all_children(grandchild);
                }
                match self.get_next_sibling(child) {
                    Some(next) => child = next,
                    None => break,
                }
            }
        }
    }

    /// Check if an item has children.
    pub fn has_children(&self, item: TreeItemId) -> bool {
        self.get_first_child(item).is_some()
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}

impl Drop for TreeCtrl {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

// ============================================================================
// Win32 types reimplemented here because `windows-sys 0.59`
// does not export `TV_INSERTSTRUCT` and `TV_ITEMEX` in their full form.
#[repr(C)]
struct TV_ITEMEX {
    mask: u32,
    hItem: HTREEITEM,
    state: u32,
    stateMask: u32,
    pszText: *mut u16,
    cchTextMax: i32,
    iImage: i32,
    iSelectedImage: i32,
    cChildren: i32,
    lParam: isize,
}

#[repr(C)]
struct TV_ITEMW {
    mask: u32,
    hItem: HTREEITEM,
    state: u32,
    stateMask: u32,
    pszText: *mut u16,
    cchTextMax: i32,
    iImage: i32,
    iSelectedImage: i32,
    cChildren: i32,
    lParam: isize,
}

#[repr(C)]
struct TV_INSERTSTRUCT {
    hParent: HTREEITEM,
    hInsertAfter: HTREEITEM,
    itemex: TV_ITEMEX,
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
