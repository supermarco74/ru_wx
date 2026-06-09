//! Tab control (notebook of pages).
//!
//! This is a thin wrapper around the Win32 `SysTabControl32` common control.
//! It exposes a "notebook" of pages, each backed by a [`Panel`]. The Tab
//! widget itself owns only the tab strip; the page content is provided by
//! the caller as a `Panel`, which the Tab positions to overlap the
//! tab control's content area and shows/hides based on the selected tab.
//!
//! ## How it works under the hood
//!
//! The Tab control is a child of the frame. Each page is a [`Panel`]
//! created as a child of the same frame, sized and positioned to fill
//! the tab control's *display area* (the rectangle inside the tab
//! control's border, below the tab strip). When the user clicks a tab,
//! the tab control sends a `TCN_SELCHANGE` notification via `WM_NOTIFY`
//! to its parent. The Tab widget listens for that notification through
//! the frame's `register_notify_handler` mechanism, updates its
//! internal selection, shows the newly-selected page (and hides all
//! others), and fires the user's `on_selection_change` callback.
//!
//! ## Notes
//!
//! - The Tab control's children are *positioned but not parented to* the
//!   tab control: the page Panels are direct children of the frame, so
//!   `WM_COMMAND` messages from controls inside a page are delivered to
//!   the frame (via the Panel's forwarding WndProc). This keeps the
//!   existing event-dispatch machinery working unchanged.
//! - The first page added is shown by default; all others are hidden.

use std::cell::RefCell;
use std::rc::Rc;

use crate::frame::Frame;
use crate::geometry::Rect;
use crate::image_list::ImageList;
use crate::panel::Panel;
use crate::widget::{Widget, WidgetRef};

#[cfg(target_os = "windows")]
use crate::platform::win32::{next_control_id, to_wide};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── Win32 Tab Control constants ────────────────────────────────────────

#[cfg(target_os = "windows")]
const TCM_FIRST: u32 = 0x1300;
#[cfg(target_os = "windows")]
const TCM_GETITEMCOUNT: u32 = TCM_FIRST + 4;
#[cfg(target_os = "windows")]
const TCM_SETIMAGELIST: u32 = TCM_FIRST + 3;
#[cfg(target_os = "windows")]
const TCM_INSERTITEM: u32 = TCM_FIRST + 7;
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const TCM_DELETEITEM: u32 = TCM_FIRST + 8;
#[cfg(target_os = "windows")]
const TCM_GETCURSEL: u32 = TCM_FIRST + 11;
#[cfg(target_os = "windows")]
const TCM_SETCURSEL: u32 = TCM_FIRST + 12;
#[cfg(target_os = "windows")]
const TCM_ADJUSTRECT: u32 = TCM_FIRST + 40;

/// `TCIF_TEXT` — `TCITEM.mask` flag: `pszText` is valid.
#[cfg(target_os = "windows")]
const TCIF_TEXT: u32 = 0x0001;
/// `TCIF_IMAGE` — `TCITEM.mask` flag: `iImage` is valid.
#[cfg(target_os = "windows")]
const TCIF_IMAGE: u32 = 0x0002;
/// `TCM_GETITEM` — retrieve information about an existing tab.
#[cfg(target_os = "windows")]
const TCM_GETITEM: u32 = TCM_FIRST + 5;

/// `TCITEMW` — the Win32 structure passed to `TCM_INSERTITEM`.
///
/// We define it locally (instead of pulling in `windows-sys`'s
/// `TCITEMW`) so the layout is stable across `windows-sys` versions.
/// The field names match the Win32 header, hence `non_snake_case`.
#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(clippy::upper_case_acronyms)]
#[allow(non_snake_case)]
struct TCITEMW {
    mask: u32,
    dwState: u32,
    dwStateMask: u32,
    pszText: *mut u16,
    cchTextMax: i32,
    iImage: i32,
    lParam: isize,
}

// ── Inner type ────────────────────────────────────────────────────────

struct TabInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    id: u16,
    rect: Rect,
    /// The `Panel` backing each page, in insertion order. We keep the
    /// full `Panel` (a cloned `Rc`) rather than just the HWND so we can
    /// re-layout the page's contents when the tab control is resized.
    #[cfg(target_os = "windows")]
    page_panels: Vec<Panel>,
    /// Index of the currently-selected page. Always 0 when no pages
    /// have been added yet.
    selected: usize,
    enabled: bool,
    visible: bool,
    /// User's `on_selection_change` callback, if any.
    on_sel_change: Option<Box<dyn FnMut(usize)>>,
}

// ── Public type ───────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Tab {
    inner: Rc<RefCell<TabInner>>,
}

impl Tab {
    /// Create a new tab control as a child of the given frame.
    pub fn new(frame: &Frame) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = frame.hwnd();
            let wide_class = to_wide("SysTabControl32");
            // WS_TABSTOP makes the tab control focusable (so keyboard
            // navigation can land on it). TCS_TABS (the default style,
            // i.e. value 0) puts the tab strip on top.
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_TABSTOP,
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

        // Force the Tab control into Unicode mode. Even with a Common
        // Controls v6 manifest, some host processes initialise the
        // control as ANSI, which truncates a UTF-16 `TCITEMW.pszText`
        // buffer to its first code unit (the high byte of "Text" is
        // 0x00, so the ANSI read stops at 1 char). `TCM_SETUNICODEFORMAT`
        // (wParam=TRUE) is documented to force the W variant regardless
        // of the process-wide A/W toggle.
        #[cfg(target_os = "windows")]
        // SAFETY: `hwnd` is a live Tab control returned by `CreateWindowExW`;
        // `SendMessageW` with `TCM_SETUNICODEFORMAT` / wParam=TRUE is the
        // documented way to force the control to its W variants. There is
        // no output buffer and no other pointer argument.
        unsafe {
            use windows_sys::Win32::UI::Controls::TCM_SETUNICODEFORMAT;
            SendMessageW(hwnd, TCM_SETUNICODEFORMAT, 1, 0);
        }

        Tab {
            inner: Rc::new(RefCell::new(TabInner {
                #[cfg(target_os = "windows")]
                hwnd,
                id,
                rect: Rect::new(0, 0, 200, 200),
                page_panels: Vec::new(),
                selected: 0,
                enabled: true,
                visible: true,
                on_sel_change: None,
            })),
        }
    }

    /// Attach an image list to the tab control. Once an image list is
    /// attached, pages added with [`Tab::add_page_with_image`] will
    /// display the icon at `image_index` in the tab strip.
    #[cfg(target_os = "windows")]
    pub fn set_image_list(&self, image_list: &ImageList) {
        let hwnd = self.inner.borrow().hwnd;
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            SendMessageW(hwnd, TCM_SETIMAGELIST, 0, image_list.handle());
        }
    }

    /// Non-Windows stub.
    #[cfg(not(target_os = "windows"))]
    pub fn set_image_list(&self, _image_list: &ImageList) {}

    /// Add a new page to the tab control.
    ///
    /// The `panel` is the container that will hold this page's content.
    /// It is automatically sized to the tab control's content area and
    /// is shown only when this page is the currently-selected tab.
    ///
    /// Returns the zero-based index of the new page.
    pub fn add_page(&self, title: &str, panel: &Panel) -> i32 {
        #[cfg(target_os = "windows")]
        {
            let mut inner = self.inner.borrow_mut();
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                let mut wide = to_wide(title);
                let mut item = TCITEMW {
                    mask: TCIF_TEXT,
                    dwState: 0,
                    dwStateMask: 0,
                    pszText: wide.as_mut_ptr(),
                    cchTextMax: wide.len() as i32,
                    iImage: -1,
                    lParam: 0,
                };

                // DIAGNOSTIC: log struct layout + the text we're about to send,
                // then read back what's actually stored via TCM_GETITEM.
                use std::io::Write;
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("c:\\Users\\marco\\Documents\\code\\test wxdragon\\tab_debug.log")
                {
                    let _ = writeln!(
                        f,
                        "[tab] add_page title=\"{}\" struct_size={} wide_len={} mask=0x{:x}",
                        title,
                        std::mem::size_of::<TCITEMW>(),
                        wide.len(),
                        TCIF_TEXT,
                    );
                }
                // Append the new tab at the end of the tab strip. Passing
                // the current page count as wParam makes Win32 insert the
                // item at the next free index (and returns that index),
                // so the tab order matches the order of `add_page` calls.
                // Previously this used `0` as the wParam, which inserted
                // each new tab at position 0 and put the tabs in reverse
                // order (and made every page appear "selected", so all
                // four pages ended up overlapping at the same location).
                let insert_at = inner.page_panels.len();
                let index = SendMessageW(
                    inner.hwnd,
                    TCM_INSERTITEM,
                    insert_at,
                    &mut item as *mut TCITEMW as isize,
                );

                // DIAGNOSTIC: read back what's actually stored so we can see
                // whether the issue is the insert or the display.
                if index >= 0 {
                    let mut buf = vec![0u16; 256];
                    let mut readback = TCITEMW {
                        mask: TCIF_TEXT,
                        dwState: 0,
                        dwStateMask: 0,
                        pszText: buf.as_mut_ptr(),
                        cchTextMax: 256,
                        iImage: 0,
                        lParam: 0,
                    };
                    let _ = SendMessageW(
                        inner.hwnd,
                        TCM_GETITEM,
                        index as usize,
                        &mut readback as *mut TCITEMW as isize,
                    );
                    if let Ok(mut f) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("c:\\Users\\marco\\Documents\\code\\test wxdragon\\tab_debug.log")
                    {
                        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
                        let text = String::from_utf16_lossy(&buf[..end]);
                        let _ = writeln!(
                            f,
                            "[tab] readback idx={} stored=\"{}\" (passed: \"{}\")",
                            index, text, title
                        );
                    }
                }

                // If index is negative, the insert failed.
                if index < 0 {
                    return -1;
                }

                let index = index as usize;

                // Drop the wide string's borrow by ending the unsafe block.
                drop(wide);

                // Store the page's panel (a cloned Rc). We keep the
                // full Panel so we can re-layout the page's sizer
                // when the tab control is resized.
                inner.page_panels.push(panel.clone());

                // Position/resize the new page over the tab control's
                // display area. If this is the first page, also show
                // it; otherwise hide it (only the selected page is
                // visible).
                let is_selected = index == inner.selected;
                inner.layout_page(panel);
                ShowWindow(panel.hwnd(), if is_selected { SW_SHOW } else { SW_HIDE });

                index as i32
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = (title, panel);
            0
        }
    }

    /// Add a new page with an icon from the attached image list.
    ///
    /// The `image_index` is the zero-based index into the image list
    /// previously attached with [`Tab::set_image_list`]. Pass `-1` to
    /// show no icon.
    ///
    /// Returns the zero-based index of the new page, or `-1` on failure.
    pub fn add_page_with_image(&self, title: &str, panel: &Panel, image_index: i32) -> i32 {
        #[cfg(target_os = "windows")]
        {
            let mut inner = self.inner.borrow_mut();
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                let mut wide = to_wide(title);
                let mut item = TCITEMW {
                    mask: TCIF_TEXT | TCIF_IMAGE,
                    dwState: 0,
                    dwStateMask: 0,
                    pszText: wide.as_mut_ptr(),
                    cchTextMax: wide.len() as i32,
                    iImage: image_index,
                    lParam: 0,
                };
                let insert_at = inner.page_panels.len();
                let index = SendMessageW(
                    inner.hwnd,
                    TCM_INSERTITEM,
                    insert_at,
                    &mut item as *mut TCITEMW as isize,
                );
                if index < 0 {
                    return -1;
                }
                let index = index as usize;

                drop(wide);

                inner.page_panels.push(panel.clone());

                let is_selected = index == inner.selected;
                inner.layout_page(panel);
                ShowWindow(panel.hwnd(), if is_selected { SW_SHOW } else { SW_HIDE });

                index as i32
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = (title, panel, image_index);
            0
        }
    }

    /// Return the number of pages currently in the tab control.
    pub fn get_page_count(&self) -> usize {
        #[cfg(target_os = "windows")]
        {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let result = unsafe { SendMessageW(self.inner.borrow().hwnd, TCM_GETITEMCOUNT, 0, 0) };
            result as usize
        }

        #[cfg(not(target_os = "windows"))]
        0
    }

    /// Return the index of the currently-selected page, or `None` if
    /// the tab control has no pages.
    pub fn get_selection(&self) -> Option<usize> {
        let inner = self.inner.borrow();
        if inner.page_panels.is_empty() {
            None
        } else {
            Some(inner.selected)
        }
    }

    /// Programmatically select the page at the given index. Shows
    /// that page and hides all the others, and fires the
    /// `on_selection_change` callback (if any).
    pub fn set_selection(&self, index: usize) {
        #[cfg(target_os = "windows")]
        {
            let mut inner = self.inner.borrow_mut();
            if index >= inner.page_panels.len() {
                return;
            }
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                SendMessageW(inner.hwnd, TCM_SETCURSEL, index, 0);
            }
            inner.selected = index;

            // Show the new page, hide all others.
            for (i, page) in inner.page_panels.iter().enumerate() {
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                unsafe {
                    ShowWindow(page.hwnd(), if i == index { SW_SHOW } else { SW_HIDE });
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = index;
        }
    }

    /// Register a callback that fires when the user selects a
    /// different tab. The callback receives the zero-based index of
    /// the newly-selected page.
    pub fn on_selection_change<F: FnMut(usize) + 'static>(&self, frame: &Frame, callback: F) {
        // Store the user's callback inside our inner state.
        self.inner.borrow_mut().on_sel_change = Some(Box::new(callback));

        // Register a WM_NOTIFY handler on the frame that:
        //   1. Queries the tab control for the new selection.
        //   2. Updates our inner state.
        //   3. Shows the new page, hides all others.
        //   4. Fires the user's callback (if any).
        let inner = self.inner.clone();
        let id = self.inner.borrow().id;
        frame.register_notify_handler(
            id,
            Box::new(move |_code| {
                #[cfg(target_os = "windows")]
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                unsafe {
                    let new_selected = {
                        let hwnd = inner.borrow().hwnd;
                        SendMessageW(hwnd, TCM_GETCURSEL, 0, 0) as usize
                    };

                    let changed = {
                        let mut inner_mut = inner.borrow_mut();
                        if new_selected == inner_mut.selected
                            || new_selected >= inner_mut.page_panels.len()
                        {
                            false
                        } else {
                            inner_mut.selected = new_selected;
                            // Show the new page, hide all others.
                            for (i, page) in inner_mut.page_panels.iter().enumerate() {
                                ShowWindow(
                                    page.hwnd(),
                                    if i == new_selected { SW_SHOW } else { SW_HIDE },
                                );
                            }
                            true
                        }
                    };

                    if changed {
                        // Fire the user's callback. Take it out,
                        // call it, put it back — the same pattern
                        // the frame uses for its command/notify
                        // dispatchers.
                        let cb = inner.borrow_mut().on_sel_change.take();
                        if let Some(mut c) = cb {
                            c(new_selected);
                            inner.borrow_mut().on_sel_change = Some(c);
                        }
                    }
                }

                #[cfg(not(target_os = "windows"))]
                {
                    let _ = inner;
                }
            }),
        );
    }

    /// Return the control's Win32 ID.
    pub fn id(&self) -> u16 {
        self.inner.borrow().id
    }

    /// Get a `WidgetRef` for use with sizers.
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

// ── internal helpers ─────────────────────────────────────────────────

impl TabInner {
    /// Position/resize `page` to exactly cover the tab control's
    /// display area (the rectangle inside the border, below the tab
    /// strip). Calling `set_size` on the panel also re-lays out any
    /// sizer installed on the page, so the page's contents follow
    /// along when the tab control is resized.
    #[cfg(target_os = "windows")]
    fn layout_page(&self, page: &Panel) {
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            // Build a RECT that describes the *outer* (full) rectangle
            // of the tab control, in its parent's coordinates.
            let mut outer: RECT = std::mem::zeroed();
            outer.left = self.rect.x;
            outer.top = self.rect.y;
            outer.right = self.rect.x + self.rect.width as i32;
            outer.bottom = self.rect.y + self.rect.height as i32;

            // TCM_ADJUSTRECT with wParam = 0 converts an outer rect
            // (full tab control, including tab strip and border) into
            // the inner display-area rect.
            SendMessageW(
                self.hwnd,
                TCM_ADJUSTRECT,
                0,
                &mut outer as *mut RECT as isize,
            );

            // After the call, `outer` is the display area.
            let w = (outer.right - outer.left).max(0) as u32;
            let h = (outer.bottom - outer.top).max(0) as u32;
            // Use the panel's set_size / set_position methods (not raw
            // MoveWindow) so the panel's sizer re-layouts with the new
            // dimensions.
            page.set_position(outer.left, outer.top);
            page.set_size(w, h);
        }
    }
}

// ── Widget trait ─────────────────────────────────────────────────────

impl Widget for TabInner {
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

            // Re-layout every page so it stays glued to the tab
            // control's display area (and so each page's sizer
            // re-lays out with the new position).
            for page in &self.page_panels {
                self.layout_page(page);
            }
        }
    }

    fn set_size(&mut self, w: u32, h: u32) {
        self.rect.width = w;
        self.rect.height = h;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(self.hwnd, self.rect.x, self.rect.y, w as i32, h as i32, 1);

            // Re-layout every page so it stays glued to the tab
            // control's display area (and so each page's sizer
            // re-lays out with the new size).
            for page in &self.page_panels {
                self.layout_page(page);
            }
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
            // Show/hide the currently-selected page along with the
            // tab control, but leave non-selected pages hidden.
            for (i, page) in self.page_panels.iter().enumerate() {
                if i == self.selected {
                    ShowWindow(page.hwnd(), if visible { SW_SHOW } else { SW_HIDE });
                }
            }
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
