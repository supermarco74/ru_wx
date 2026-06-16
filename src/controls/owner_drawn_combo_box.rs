//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Owner-drawn combo box (`wxOwnerDrawnComboBox`).
//!
//! Wraps a standard Win32 combo with `CBS_OWNERDRAWFIXED` for custom item painting.

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::geometry::Rect;
use crate::core::widget::{Widget, WidgetRef, Window};
use crate::window::frame::{DrawItemRequest, Frame, MeasureItemRequest};

use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{
    CreateSolidBrush, DeleteObject, DrawTextW, FillRect, GetSysColor, SetBkColor, SetTextColor,
    DT_END_ELLIPSIS, DT_LEFT, DT_SINGLELINE, DT_VCENTER,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[cfg(target_os = "windows")]
const CBS_OWNERDRAWFIXED: u32 = 0x0010;
#[cfg(target_os = "windows")]
const CBS_DROPDOWNLIST: u32 = 0x0003;
#[cfg(target_os = "windows")]
const CB_ADDSTRING: u32 = 0x0143;
#[cfg(target_os = "windows")]
const COLOR_HIGHLIGHT: i32 = 13;
#[cfg(target_os = "windows")]
const COLOR_HIGHLIGHTTEXT: i32 = 14;
#[cfg(target_os = "windows")]
const COLOR_WINDOW: i32 = 5;
#[cfg(target_os = "windows")]
const COLOR_WINDOWTEXT: i32 = 8;

/// Optional custom paint callback for [`OwnerDrawnComboBox`].
type OwnerDrawFn = Box<dyn FnMut(&DrawItemRequest, &str)>;

struct OwnerDrawnComboBoxInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    rect: Rect,
    items: Vec<String>,
    visible: bool,
    custom_draw: RefCell<Option<OwnerDrawFn>>,
}

#[derive(Clone)]
pub struct OwnerDrawnComboBox {
    inner: Rc<RefCell<OwnerDrawnComboBoxInner>>,
}

impl OwnerDrawnComboBox {
    pub fn new<W: Window>(parent: &W) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        let hwnd = unsafe {
            let wide_class = to_wide("COMBOBOX");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | CBS_DROPDOWNLIST | CBS_OWNERDRAWFIXED,
                0,
                0,
                160,
                200,
                parent.hwnd(),
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = parent;

        Self {
            inner: Rc::new(RefCell::new(OwnerDrawnComboBoxInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                rect: Rect::new(0, 0, 160, 24),
                items: Vec::new(),
                visible: true,
                custom_draw: RefCell::new(None),
            })),
        }
    }

    pub fn append(&self, label: &str) {
        let index = {
            let mut inner = self.inner.borrow_mut();
            inner.items.push(label.to_string());
            inner.items.len() - 1
        };
        #[cfg(target_os = "windows")]
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            SendMessageW(hwnd, CB_ADDSTRING, 0, index as isize);
        }
        let _ = index;
    }

    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    /// Register optional custom painting; default GDI text drawing is used otherwise.
    pub fn on_draw_item<F: FnMut(&DrawItemRequest, &str) + 'static>(&self, f: F) {
        *self.inner.borrow().custom_draw.borrow_mut() = Some(Box::new(f));
    }

    /// Wire `WM_DRAWITEM` / `WM_MEASUREITEM` handlers on `frame`.
    pub fn attach_to_frame(&self, frame: &Frame) {
        let inner = self.inner.clone();
        let id = self.id();
        frame.register_measure_item_handler(
            id,
            Box::new(|_req: MeasureItemRequest| 22),
        );
        frame.register_draw_item_handler(
            id,
            Box::new(move |req: DrawItemRequest| {
                let label = inner
                    .borrow()
                    .items
                    .get(req.index as usize)
                    .cloned()
                    .unwrap_or_default();
                if let Some(ref mut custom) = *inner.borrow().custom_draw.borrow_mut() {
                    custom(&req, &label);
                    return;
                }
                #[cfg(target_os = "windows")]
                default_draw_item(&req, &label);
                #[cfg(not(target_os = "windows"))]
                let _ = (&req, label);
            }),
        );
    }

    pub fn on_selection_change(&self, frame: &Frame, mut f: impl FnMut(usize) + 'static) {
        frame.register_command_handler(self.id(), Box::new(move || f(0)));
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

#[cfg(target_os = "windows")]
fn default_draw_item(req: &DrawItemRequest, label: &str) {
    use crate::platform::win32::to_wide;
    // SAFETY: GDI drawing in the owner-draw item HDC provided by the framework.
    unsafe {
        let hdc = req.hdc as HDC;
        let bg = if req.selected {
            GetSysColor(COLOR_HIGHLIGHT) as u32
        } else {
            GetSysColor(COLOR_WINDOW) as u32
        };
        let fg = if req.selected {
            GetSysColor(COLOR_HIGHLIGHTTEXT) as u32
        } else {
            GetSysColor(COLOR_WINDOWTEXT) as u32
        };
        let brush = CreateSolidBrush(bg);
        let mut rc = RECT {
            left: req.rect.x,
            top: req.rect.y,
            right: req.rect.x + req.rect.width as i32,
            bottom: req.rect.y + req.rect.height as i32,
        };
        FillRect(hdc, &rc, brush);
        DeleteObject(brush);
        SetBkColor(hdc, bg);
        SetTextColor(hdc, fg);
        let mut wide = to_wide(label);
        DrawTextW(
            hdc,
            wide.as_mut_ptr(),
            -1,
            &mut rc,
            DT_SINGLELINE | DT_VCENTER | DT_LEFT | DT_END_ELLIPSIS,
        );
    }
}

impl Widget for OwnerDrawnComboBoxInner {
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
        unsafe {
            ShowWindow(self.hwnd, if visible { SW_SHOW } else { SW_HIDE });
        }
    }

    fn is_enabled(&self) -> bool {
        true
    }

    fn set_enabled(&mut self, _enabled: bool) {}
}

#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::HDC;
