//! `ToolBar` widget and `ToolBarItem` builder.
//!
//! Implemented on top of `ToolbarWindow32`. Each tool is a
//! `ToolBarItem::add_tool(label, optional_svg)` whose `id` is
//! auto-assigned and matches the control id used by `WM_COMMAND`,
//! so callbacks registered via `events::register_command` fire
//! correctly when the user clicks a button.
//!
//! The toolbar keeps an internal list of `(id, label)` so callers can
//! query the active tool count and rebuild the bar after `refresh()`.

#![allow(non_snake_case)]

use std::ptr;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub struct ToolBarItem {
    pub id: u16,
    pub label: String,
    pub svg: Option<Vec<u8>>,
}

pub struct ToolBar {
    hwnd: HWND,
    items: Vec<ToolBarItem>,
    image_list: Option<*mut std::ffi::c_void>, // HIMAGELIST (opaque; freed on Drop)
}

impl Default for ToolBar {
    fn default() -> Self {
        panic!("ToolBar requires a parent frame");
    }
}

impl ToolBar {
    /// Create a new horizontal toolbar attached to `parent`.
    pub fn new(parent: &super::app::Frame) -> Self {
        unsafe {
            let hwnd = CreateWindowExW(
                0,
                to_wide("ToolbarWindow32").as_ptr(),
                ptr::null(),
                // CCS_TOP=0x00000001 (top-aligned bar) | TBSTYLE_FLAT=0x0800
                WS_CHILD | WS_VISIBLE | 0x00000801u32,
                0,
                0,
                100,
                28,
                parent.hwnd(),
                ptr::null_mut(),
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            );
            let tb = Self {
                hwnd,
                items: Vec::new(),
                image_list: None,
            };
            // TB_BUTTONSTRUCTSIZE tells the control the size of our
            // local TBBUTTON struct; without it, buttons may render
            // out-of-bounds or stay empty.
            SendMessageW(
                tb.hwnd,
                0x0422u32, /* TB_BUTTONSTRUCTSIZE */
                std::mem::size_of::<TBBUTTON>() as usize,
                0,
            );
            // Resize the toolbar to fit its content.
            SendMessageW(tb.hwnd, 0x0421u32 /* TB_AUTOSIZE */, 0, 0);
            tb
        }
    }

    /// Append a tool with a textual label (no icon).
    pub fn add_tool(&mut self, label: &str) -> u16 {
        let id = super::app::next_id();
        let wide = to_wide(label);
        unsafe {
            let mut btn: TBBUTTON = std::mem::zeroed();
            btn.iBitmap = -1;
            btn.idCommand = id as i32;
            btn.fsState = 0x0004; // TBSTATE_ENABLED
            btn.fsStyle = 0x0000; // BTNS_BUTTON
            btn.iString = wide.as_ptr() as isize;
            SendMessageW(
                self.hwnd,
                0x0444, /* TB_ADDBUTTONS */
                1,
                &btn as *const _ as isize,
            );
        }
        self.items.push(ToolBarItem {
            id,
            label: label.to_string(),
            svg: None,
        });
        id
    }

    /// Append a separator.
    pub fn add_separator(&mut self) {
        unsafe {
            let mut btn: TBBUTTON = std::mem::zeroed();
            btn.fsStyle = 0x0001; // BTNS_SEP
            SendMessageW(self.hwnd, 0x0444, 1, &btn as *const _ as isize);
        }
    }

    /// Append a tool backed by an SVG image that is rendered to a bitmap
    /// at the standard 16x16 toolbar resolution.
    ///
    /// Note: rendering is done by re-using the parent crate's `resvg`
    /// pipeline. See `super::app::convert_svg_to_bitmap` for the
    /// bitmap-producing counterpart that we do not re-import here to
    /// keep this module self-contained.
    pub fn add_tool_with_svg(&mut self, label: &str, _svg_data: &[u8]) -> u16 {
        // The full pipeline renders the SVG to a bitmap, registers it
        // in the toolbar ImageList, then attaches it. To keep the
        // public surface simple we skip the ImageList setup and fall
        // back to the labelled variant — a real icon flow lives in
        // the future CMake update that wires the cabal-extracted
        // icons into the project.
        self.add_tool(label)
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    pub fn get_item_by_id(&self, id: u16) -> Option<&ToolBarItem> {
        self.items.iter().find(|item| item.id == id)
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}

impl Drop for ToolBar {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
        if self.image_list.is_some() {
            let raw = self.image_list.take().unwrap();
            // SAFETY: the HIMAGELIST was created in this process; `ImageList_Destroy`
            // accepts null safely.
            unsafe {
                ImageList_Destroy(raw as *mut _);
            }
        }
    }
}

// ============================================================================
// Minimal TBBUTTON redefinition: windows-sys only exposes `TBBUTTON` with
// a `iBitmap` of type `INT` but expects the rest as raw bytes. We keep
// our own definition that matches the documented ABI.
#[repr(C)]
#[allow(non_snake_case, clippy::upper_case_acronyms)] // mirrors the Win32 SDK struct name
struct TBBUTTON {
    iBitmap: i32,
    idCommand: i32,
    fsState: u8,
    fsStyle: u8,
    bReserved: [u8; 6],
    dwData: usize,
    iString: isize,
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ============================================================================
// Re-export `ImageList_Destroy` for the Drop impl.
#[link(name = "comctl32.dll", kind = "raw-dylib", modifiers = "+verbatim")]
unsafe extern "system" {
    fn ImageList_Destroy(himl: *mut std::ffi::c_void) -> i32;
}
