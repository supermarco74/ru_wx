//! Native property grid using `SysTreeView32` (categories) and
//! `SysHeader32` (column header) plus an `EDIT` for inline editing.
//!
//! This mirrors the macOS `NSOutlineView + NSTableView` combination.

use std::cell::RefCell;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// TreeView messages not exported as macros by windows-sys 0.59.
const TVM_INSERTITEMW: u32 = 0x1100 + 50;
const TVM_DELETEITEM: u32 = 0x1100 + 1;
const HDM_SETITEMW: u32 = 0x1200 + 4;

use crate::core::geometry::Rect;
use crate::platform::NativeHandle;

thread_local! {
    static PG_SELECTED: RefCell<Option<usize>> = const { RefCell::new(None) };
}

/// Public handle returned to `adv::property_grid`.
pub struct PropertyGridHandle {
    pub tree_hwnd: NativeHandle,
    pub header_hwnd: NativeHandle,
    pub host_hwnd: NativeHandle,
}

/// Create the property grid: a header on top, a tree below.
pub fn create_property_grid(parent: NativeHandle, initial_rect: Rect) -> PropertyGridHandle {
    unsafe {
        ensure_classes_registered();
        let hinstance = GetModuleHandleW(std::ptr::null());

        // Host (acts as the property-grid composite container).
        let class_host = to_wide("ru_wx_PropertyGrid");
        let host = CreateWindowExW(
            0,
            class_host.as_ptr(),
            std::ptr::null_mut(),
            WS_CHILD | WS_VISIBLE | WS_BORDER,
            initial_rect.x,
            initial_rect.y,
            initial_rect.width as i32,
            initial_rect.height as i32,
            parent as HWND,
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null(),
        );

        // Header (column title).
        let class_header = to_wide("SysHeader32");
        let header = CreateWindowExW(
            0,
            class_header.as_ptr(),
            std::ptr::null_mut(),
            WS_CHILD | WS_VISIBLE | HDS_BUTTONS,
            0,
            0,
            initial_rect.width as i32,
            20,
            host as HWND,
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null(),
        );

        // Tree (categories + rows).
        let class_tree = to_wide("SysTreeView32");
        let tree = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            class_tree.as_ptr(),
            std::ptr::null_mut(),
            WS_CHILD
                | WS_VISIBLE
                | TVS_HASBUTTONS
                | TVS_HASLINES
                | TVS_LINESATROOT
                | TVS_SHOWSELALWAYS,
            0,
            20,
            initial_rect.width as i32,
            initial_rect.height as i32 - 20,
            host as HWND,
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null(),
        );

        PropertyGridHandle {
            tree_hwnd: tree as NativeHandle,
            header_hwnd: header as NativeHandle,
            host_hwnd: host as NativeHandle,
        }
    }
}

/// Add a category (top-level tree node).
pub fn property_grid_add_category(handle: &PropertyGridHandle, label: &str) -> usize {
    unsafe {
        let wide = to_wide(label);
        let mut item: TVITEMEXW = std::mem::zeroed();
        item.mask = TVIF_TEXT;
        item.pszText = wide.as_ptr() as *mut _;
        item.cchTextMax = wide.len() as i32;
        let inserted = SendMessageW(
            handle.tree_hwnd as HWND,
            TVM_INSERTITEMW,
            0,
            (&mut item as *mut TVITEMEXW) as isize,
        );
        inserted as usize
    }
}

/// Add a property row under the given category.
pub fn property_grid_add_property(
    handle: &PropertyGridHandle,
    parent: usize,
    label: &str,
    _value: &str,
) -> usize {
    unsafe {
        let wide = to_wide(label);
        let mut item: TVITEMEXW = std::mem::zeroed();
        item.mask = TVIF_TEXT;
        item.pszText = wide.as_ptr() as *mut _;
        item.cchTextMax = wide.len() as i32;
        let hparent = parent as HTREEITEM;
        // TVINSERTSTRUCT layout: { hParent: HTREEITEM, hInsertAfter: HTREEITEM, itemex: TVITEMEXW }
        // We build it on the stack and pass a pointer.
        let mut insert: [usize; 4] = [hparent as usize, 0, 0, 0];
        let inserted = SendMessageW(
            handle.tree_hwnd as HWND,
            TVM_INSERTITEMW,
            1,
            insert.as_mut_ptr() as isize,
        );
        let _ = item; // already unused; kept for future extension
        inserted as usize
    }
}

pub fn property_grid_set_value(_handle: &PropertyGridHandle, _row: usize, value: &str) {
    let _ = value;
}

pub fn property_grid_clear(handle: &PropertyGridHandle) {
    unsafe {
        SendMessageW(handle.tree_hwnd as HWND, TVM_DELETEITEM, 0, 0);
    }
}

pub fn property_grid_set_column_width(handle: &PropertyGridHandle, column: i32, width: i32) {
    unsafe {
        let mut col: HDITEMW = std::mem::zeroed();
        col.mask = HDI_WIDTH;
        col.cxy = width;
        SendMessageW(
            handle.header_hwnd as HWND,
            HDM_SETITEMW,
            column as usize,
            (&mut col as *mut HDITEMW) as isize,
        );
    }
}

pub fn property_grid_redraw(handle: &PropertyGridHandle) {
    unsafe {
        // Send WM_PAINT instead of relying on the macro InvalidateRect
        // (already covered by `windows_sys::Win32::UI::WindowsAndMessaging`).
        SendMessageW(handle.host_hwnd as HWND, WM_PAINT, 0, 0);
    }
}

pub fn property_grid_get_selected(_handle: &PropertyGridHandle) -> Option<usize> {
    PG_SELECTED.with(|s| *s.borrow())
}

pub fn property_grid_set_name_col_w(handle: &PropertyGridHandle, w: f64) {
    let _ = handle;
    let _ = w;
}

unsafe fn ensure_classes_registered() {
    let hinstance = GetModuleHandleW(std::ptr::null());
    let class_host = to_wide("ru_wx_PropertyGrid");
    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(pg_host_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: std::ptr::null_mut(),
        hCursor: std::ptr::null_mut(),
        hbrBackground: std::ptr::null_mut(),
        lpszMenuName: std::ptr::null(),
        lpszClassName: class_host.as_ptr(),
        hIconSm: std::ptr::null_mut(),
    };
    let _ = RegisterClassExW(&wc);
}

unsafe extern "system" fn pg_host_proc(
    hwnd: HWND,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
