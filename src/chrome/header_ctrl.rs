//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! List header control (`wxHeaderCtrl`).

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::geometry::Rect;
use crate::core::header_events::{HeaderButtonClickEvent, HeaderColumnEvent};
use crate::core::widget::{Widget, Window};
use crate::window::frame::Frame;

use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Controls::{
    HDI_FORMAT, HDI_TEXT, HDI_WIDTH, HDF_LEFT, HDF_STRING, HDITEMW, HDN_ITEMCLICKW,
    HDS_BUTTONS, WC_HEADER,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Replaceable event callback slot used by [`HeaderCtrl`].
type HeaderHandler<E> = RefCell<Option<Box<dyn FnMut(&E)>>>;

struct HeaderCtrlInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    columns: Vec<HeaderColumn>,
    rect: Rect,
    sort_column: Option<usize>,
    on_button_click: HeaderHandler<HeaderButtonClickEvent>,
    on_column_event: HeaderHandler<HeaderColumnEvent>,
}

/// Column header bar (`wxHeaderCtrl`).
pub struct HeaderCtrl {
    inner: Rc<RefCell<HeaderCtrlInner>>,
}

/// One sortable column (`wxHeaderColumn`).
#[derive(Debug, Clone)]
pub struct HeaderColumn {
    pub title: String,
    pub width: u32,
    pub visible: bool,
}

impl HeaderCtrl {
    /// Logical-only header (no native HWND) for tests and off-screen use.
    pub fn new(rect: Rect) -> Self {
        Self {
            inner: Rc::new(RefCell::new(HeaderCtrlInner {
                #[cfg(target_os = "windows")]
                hwnd: std::ptr::null_mut(),
                id: 0,
                columns: Vec::new(),
                rect,
                sort_column: None,
                on_button_click: RefCell::new(None),
                on_column_event: RefCell::new(None),
            })),
        }
    }

    /// Create a native `SysHeader32` control attached to `parent`.
    pub fn with_parent<W: Window>(parent: &W, rect: Rect) -> Self {
        let id = next_control_id();
        #[cfg(target_os = "windows")]
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                WC_HEADER,
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | HDS_BUTTONS,
                rect.x,
                rect.y,
                rect.width as i32,
                rect.height as i32,
                parent.hwnd(),
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        #[cfg(not(target_os = "windows"))]
        let _ = parent;

        Self {
            inner: Rc::new(RefCell::new(HeaderCtrlInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                columns: Vec::new(),
                rect,
                sort_column: None,
                on_button_click: RefCell::new(None),
                on_column_event: RefCell::new(None),
            })),
        }
    }

    /// Wire `HDN_*` notifications through the parent frame's message loop.
    pub fn attach_to_frame(&self, frame: &Frame) {
        let id = self.inner.borrow().id;
        if id == 0 {
            return;
        }
        let inner = self.inner.clone();
        frame.register_notify_handler(id, Box::new(move |code| {
            #[cfg(target_os = "windows")]
            if code == HDN_ITEMCLICKW {
                if let Some(ref mut cb) = *inner.borrow().on_button_click.borrow_mut() {
                    cb(&HeaderButtonClickEvent::new(0));
                }
            }
            #[cfg(not(target_os = "windows"))]
            let _ = code;
        }));
    }

    pub fn on_column_event<F: FnMut(&HeaderColumnEvent) + 'static>(&self, f: F) {
        *self.inner.borrow().on_column_event.borrow_mut() = Some(Box::new(f));
    }

    pub fn resize_column(&self, column: usize, width: u32) {
        self.set_column_width(column, width);
        if let Some(ref mut cb) = *self.inner.borrow().on_column_event.borrow_mut() {
            cb(&HeaderColumnEvent::new(column, width));
        }
    }

    pub fn on_button_click<F: FnMut(&HeaderButtonClickEvent) + 'static>(&self, f: F) {
        *self.inner.borrow().on_button_click.borrow_mut() = Some(Box::new(f));
    }

    pub fn click_column(&self, column: usize) {
        if let Some(ref mut cb) = *self.inner.borrow().on_button_click.borrow_mut() {
            cb(&HeaderButtonClickEvent::new(column));
        }
    }

    pub fn append_column(&mut self, title: &str, width: u32) -> usize {
        let index = {
            let mut inner = self.inner.borrow_mut();
            inner.columns.push(HeaderColumn {
                title: title.to_string(),
                width,
                visible: true,
            });
            inner.columns.len() - 1
        };
        self.insert_native_column(index, title, width);
        index
    }

    fn insert_native_column(&self, index: usize, title: &str, width: u32) {
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            if hwnd.is_null() {
                return;
            }
            let mut wide = to_wide(title);
            let mut item = HDITEMW {
                mask: HDI_TEXT | HDI_WIDTH | HDI_FORMAT,
                cxy: width as i32,
                pszText: wide.as_mut_ptr(),
                hbm: std::ptr::null_mut(),
                cchTextMax: wide.len() as i32,
                fmt: HDF_STRING | HDF_LEFT,
                lParam: 0,
                iImage: -1,
                iOrder: index as i32,
                r#type: 0,
                pvFilter: std::ptr::null_mut(),
                state: 0,
            };
            // SAFETY: valid header HWND and HDITEMW buffer.
            unsafe {
                SendMessageW(
                    hwnd,
                    HDM_INSERTITEMW,
                    index,
                    &mut item as *mut _ as LPARAM,
                );
            }
        }
        #[cfg(not(target_os = "windows"))]
        let _ = (index, title, width);
    }

    fn set_column_width(&self, column: usize, width: u32) {
        if let Some(col) = self.inner.borrow_mut().columns.get_mut(column) {
            col.width = width;
        }
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            if hwnd.is_null() {
                return;
            }
            let mut item = HDITEMW {
                mask: HDI_WIDTH,
                cxy: width as i32,
                pszText: std::ptr::null_mut(),
                hbm: std::ptr::null_mut(),
                cchTextMax: 0,
                fmt: 0,
                lParam: 0,
                iImage: 0,
                iOrder: 0,
                r#type: 0,
                pvFilter: std::ptr::null_mut(),
                state: 0,
            };
            // SAFETY: valid header HWND and HDITEMW for width update.
            unsafe {
                SendMessageW(
                    hwnd,
                    HDM_SETITEMW,
                    column,
                    &mut item as *mut _ as LPARAM,
                );
            }
        }
    }

    pub fn set_sort_column(&mut self, index: Option<usize>) {
        self.inner.borrow_mut().sort_column = index;
    }

    pub fn sort_column(&self) -> Option<usize> {
        self.inner.borrow().sort_column
    }

    pub fn columns(&self) -> Vec<HeaderColumn> {
        self.inner.borrow().columns.clone()
    }

    pub fn rect(&self) -> Rect {
        self.inner.borrow().rect
    }
}

#[cfg(target_os = "windows")]
const HDM_INSERTITEMW: u32 = 4618;
#[cfg(target_os = "windows")]
const HDM_SETITEMW: u32 = 4620;

impl Widget for HeaderCtrlInner {
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
        if !self.hwnd.is_null() {
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
    }

    fn set_size(&mut self, w: u32, h: u32) {
        self.rect.width = w;
        self.rect.height = h;
        #[cfg(target_os = "windows")]
        if !self.hwnd.is_null() {
            unsafe {
                MoveWindow(
                    self.hwnd,
                    self.rect.x,
                    self.rect.y,
                    w as i32,
                    h as i32,
                    1,
                );
            }
        }
    }

    fn rect(&self) -> Rect {
        self.rect
    }

    fn is_visible(&self) -> bool {
        true
    }

    fn set_visible(&mut self, visible: bool) {
        #[cfg(target_os = "windows")]
        if !self.hwnd.is_null() {
            unsafe {
                ShowWindow(self.hwnd, if visible { SW_SHOW } else { SW_HIDE });
            }
        }
        let _ = visible;
    }

    fn is_enabled(&self) -> bool {
        true
    }

    fn set_enabled(&mut self, _enabled: bool) {}
}
