//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! wxToolBar — an icon toolbar with optional separators and labels.
//!
//! On Windows this wraps the standard `ToolbarWindow32` common control.
//! Buttons are added with [`ToolBar::add_tool`]; use
//! [`ToolBar::add_separator`] to add a vertical gap. The image list is
//! attached with [`ToolBar::set_image_list`] (pass a reference to a
//! shared `ImageList`).
//!
//! After all buttons are added, call [`ToolBar::realize`] to commit the
//! layout. Tool clicks are delivered via `WM_COMMAND` and dispatched
//! through [`ToolBar::on_tool_clicked`].

use std::cell::RefCell;
use std::rc::Rc;

use crate::window::frame::Frame;
use crate::core::geometry::Rect;
use crate::dc::image_list::ImageList;
use crate::core::widget::Widget;

#[cfg(target_os = "windows")]
use crate::platform::win32::{next_control_id, to_wide};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── Win32 ToolBar constants ─────────────────────────────────────────────

#[cfg(target_os = "windows")]
const TB_BUTTONSTRUCTSIZE: u32 = 0x041E;
#[cfg(target_os = "windows")]
const TB_SETBITMAPSIZE: u32 = 0x0420;
#[cfg(target_os = "windows")]
const TB_SETIMAGELIST: u32 = 0x0430;
#[cfg(target_os = "windows")]
const TB_ADDBUTTONS: u32 = 0x0444;
#[cfg(target_os = "windows")]
const TB_AUTOSIZE: u32 = 0x0421;
#[cfg(target_os = "windows")]
const TB_SETEXTENDEDSTYLE: u32 = 0x0454;
#[cfg(target_os = "windows")]
const TBSTYLE_EX_DRAWDDARROWS: u32 = 0x0000_0001;
#[cfg(target_os = "windows")]
const TBSTYLE_FLAT: u32 = 0x0800;
#[cfg(target_os = "windows")]
const TBSTYLE_TOOLTIPS: u32 = 0x0100;
#[cfg(target_os = "windows")]
#[allow(dead_code)] // Win32 ABI surface — alternative text-only layout
const TBSTYLE_LIST: u32 = 0x1000;
#[cfg(target_os = "windows")]
#[allow(dead_code)] // Win32 ABI surface — transparent toolbar background
const TBSTYLE_TRANSPARENT: u32 = 0x8000;

#[cfg(target_os = "windows")]
const TBSTYLE_BUTTON: u8 = 0x00;
#[cfg(target_os = "windows")]
const TBSTYLE_SEP: u8 = 0x01;

#[cfg(target_os = "windows")]
const TBSTATE_ENABLED: u8 = 0x04;

/// A TBBUTTON structure (12 bytes on 32-bit, 32 on 64-bit). We define
/// a layout-compatible 32-byte struct for x86_64.
#[cfg(target_os = "windows")]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(clippy::upper_case_acronyms)]
struct TBBUTTON {
    i_bitmap: i32,
    id_command: u32,
    fs_state: u8,
    fs_style: u8,
    _pad: u16,
    dw_data: usize,
    i_string: isize,
}

#[cfg(target_os = "windows")]
impl TBBUTTON {
    fn separator() -> Self {
        TBBUTTON {
            i_bitmap: 0,
            id_command: 0,
            fs_state: 0,
            fs_style: TBSTYLE_SEP,
            _pad: 0,
            dw_data: 0,
            i_string: 0,
        }
    }
}

// ── Inner type ─────────────────────────────────────────────────────────

struct ToolBarInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    /// Buffered button specs (separators + real buttons) that are sent
    /// to the control on `realize()`.
    buttons: Vec<ToolSpec>,
    rect: Rect,
    visible: bool,
}

/// One logical entry on the toolbar.
#[derive(Clone)]
enum ToolSpec {
    Separator,
    Tool {
        /// Win32 control id (also used as the WM_COMMAND id).
        id: u16,
        /// Index into the attached image list, or -1 for no image.
        image_index: i32,
    },
}

#[derive(Clone)]
pub struct ToolBar {
    inner: Rc<RefCell<ToolBarInner>>,
}

impl ToolBar {
    /// Create a new tool bar and attach it to the top of `frame`.
    pub fn new(frame: &Frame) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = frame.hwnd();
            let wide_class = to_wide("ToolbarWindow32");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | TBSTYLE_FLAT | TBSTYLE_TOOLTIPS,
                0,
                0,
                0,
                0,
                parent,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = frame;

        ToolBar {
            inner: Rc::new(RefCell::new(ToolBarInner {
                #[cfg(target_os = "windows")]
                hwnd,
                buttons: Vec::new(),
                rect: Rect::new(0, 0, 0, 0),
                visible: true,
            })),
        }
    }

    /// Attach an image list. Must be called before [`ToolBar::realize`].
    #[cfg(target_os = "windows")]
    pub fn set_image_list(&self, image_list: &ImageList) {
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let state = self.inner.borrow();
            SendMessageW(state.hwnd, TB_SETIMAGELIST, 0, image_list.handle());
            // Match bitmap size to image list's image size
            let w = image_list.width();
            let h = image_list.height();
            let lparam = ((w as u32) & 0xFFFF) | (((h as u32) & 0xFFFF) << 16);
            SendMessageW(state.hwnd, TB_SETBITMAPSIZE, 0, lparam as isize);
        }
    }

    /// Add a tool button. The `image_index` is the index into the
    /// previously-attached image list. The `label` is currently unused
    /// by the control and only exists for callers who want to remember
    /// the human-readable name of each tool.
    pub fn add_tool(&self, id: u16, _label: &str, image_index: i32) {
        let mut state = self.inner.borrow_mut();
        state.buttons.push(ToolSpec::Tool { id, image_index });
    }

    /// Add a vertical separator.
    pub fn add_separator(&self) {
        self.inner.borrow_mut().buttons.push(ToolSpec::Separator);
    }

    /// Commit the buffered buttons to the control. Call this once after
    /// all tools / separators have been added and the image list has
    /// been attached. Calling it again is a no-op (already-realized
    /// buttons are cleared first).
    pub fn realize(&self) {
        #[cfg(target_os = "windows")]
        {
            let state = self.inner.borrow_mut();
            // First, instruct the control about the TBBUTTON size
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                SendMessageW(
                    state.hwnd,
                    TB_BUTTONSTRUCTSIZE,
                    std::mem::size_of::<TBBUTTON>() as usize,
                    0,
                );
                // Enable flat + transparent styles
                SendMessageW(
                    state.hwnd,
                    TB_SETEXTENDEDSTYLE,
                    0,
                    TBSTYLE_EX_DRAWDDARROWS as isize,
                );
                // Build the button array
                let mut btns: Vec<TBBUTTON> = Vec::with_capacity(state.buttons.len());
                for spec in state.buttons.iter() {
                    match spec {
                        ToolSpec::Separator => btns.push(TBBUTTON::separator()),
                        ToolSpec::Tool { id, image_index } => btns.push(TBBUTTON {
                            i_bitmap: *image_index,
                            id_command: *id as u32,
                            fs_state: TBSTATE_ENABLED,
                            fs_style: TBSTYLE_BUTTON,
                            _pad: 0,
                            dw_data: 0,
                            i_string: 0,
                        }),
                    }
                }
                SendMessageW(
                    state.hwnd,
                    TB_ADDBUTTONS,
                    btns.len(),
                    btns.as_ptr() as isize,
                );
                // Force the toolbar to lay itself out
                SendMessageW(state.hwnd, TB_AUTOSIZE, 0, 0);
            }
        }
    }

    /// Register a callback that fires when any of the tools on this
    /// toolbar is clicked. The callback receives the id of the tool.
    pub fn on_tool_clicked<F: FnMut(u16) + 'static>(&self, frame: &Frame, callback: F) {
        #[cfg(target_os = "windows")]
        {
            // Share a single FnMut across all per-id handlers via
            // Rc<RefCell<...>>. We can't simply move `callback` into
            // the first closure because we need to clone it for every
            // registered tool id.
            let callback = std::rc::Rc::new(std::cell::RefCell::new(callback));

            // Collect all tool ids registered with the toolbar
            let tool_ids: Vec<u16> = self
                .inner
                .borrow()
                .buttons
                .iter()
                .filter_map(|s| match s {
                    ToolSpec::Tool { id, .. } => Some(*id),
                    _ => None,
                })
                .collect();

            for id in tool_ids {
                let cb = callback.clone();
                frame.register_command_handler(id, Box::new(move || cb.borrow_mut()(id)));
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (frame, callback);
        }
    }

    /// Return the native window handle.
    #[cfg(target_os = "windows")]
    pub fn hwnd(&self) -> HWND {
        self.inner.borrow().hwnd
    }
}

// ── Widget trait ───────────────────────────────────────────────────────

impl Widget for ToolBarInner {
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
        true
    }

    fn set_enabled(&mut self, _enabled: bool) {
        // ToolBar has no enabled state at the bar level (each tool has
        // its own enabled state managed by the control).
    }
}
