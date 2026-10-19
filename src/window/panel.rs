//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Generic child window (`wxPanel`).
//!
//! On Windows a `Panel` is a child window of class `RuWxPanelClass`
//! (registered at first construction). It exists primarily as a
//! container that:
//!
//! * can host an automatic sizer, and
//! * can repaint a custom background colour,
//!
//! without having to derive a full new frame. Use [`Panel::new`] to
//! create one and [`Panel::set_sizer`] to attach a sizer.

#[cfg(target_os = "macos")]
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

use crate::containers::sizer::BoxSizer;
use crate::core::geometry::{Colour, Rect};
use crate::core::widget::{Widget, WidgetRef, Window};
use crate::window::frame::Frame;

#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

struct PanelInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    stub_handle: isize,
    #[cfg(target_os = "macos")]
    stub_handle: isize,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    parent_handle: crate::platform::NativeHandle,
    rect: Rect,
    widgets: Vec<WidgetRef>,
    background_colour: Colour,
    enabled: bool,
    visible: bool,
    /// Optional sizer that lays out this panel's children. When `set_size`
    /// is called, the sizer is re-laid out with the new dimensions (this
    /// is what allows tab pages to reflow when the tab control is
    /// resized).
    sizer: Option<BoxSizer>,
}

#[derive(Clone)]
pub struct Panel {
    inner: Rc<RefCell<PanelInner>>,
}

/// Register the panel window class (idempotent)
#[cfg(target_os = "windows")]
fn register_panel_class() {
    // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
    unsafe {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_name = to_wide("RuWxPanel");

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(panel_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: std::ptr::null_mut() as HBRUSH,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wc);
    }
}

impl Panel {
    /// Create a new panel as a child of the given frame
    pub fn new(frame: &Frame) -> Self {
        #[cfg(target_os = "windows")]
        register_panel_class();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = crate::platform::as_hwnd(frame.hwnd());
            let wide_class = to_wide("RuWxPanel");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN,
                0,
                0,
                200,
                200,
                parent,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(target_os = "macos")]
        let parent_handle = frame.hwnd();
        #[cfg(target_os = "macos")]
        let stub_handle =
            crate::platform::appkit::create_panel(parent_handle, Rect::new(0, 0, 200, 200));
        #[cfg(target_os = "linux")]
        let parent_handle = frame.hwnd();
        #[cfg(target_os = "linux")]
        let stub_handle =
            crate::platform::gtk::create_panel(parent_handle, Rect::new(0, 0, 200, 200));

        let panel = Panel {
            inner: Rc::new(RefCell::new(PanelInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(all(
                    not(target_os = "windows"),
                    not(target_os = "macos"),
                    not(target_os = "linux")
                ))]
                stub_handle: crate::platform::stub_backend::alloc_widget_handle(),
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                stub_handle,
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                parent_handle,
                rect: Rect::new(0, 0, 200, 200),
                widgets: Vec::new(),
                background_colour: Colour::LIGHT_GREY,
                enabled: true,
                visible: true,
                sizer: None,
            })),
        };

        // Store the Rc pointer in GWLP_USERDATA for WndProc access
        #[cfg(target_os = "windows")]
        {
            let inner_clone = panel.inner.clone();
            let raw = Rc::into_raw(inner_clone);
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw as isize);
            }
        }

        #[cfg(target_os = "macos")]
        register_panel_macos(stub_handle, panel.inner.clone());

        panel
    }

    /// Create a new panel as a child of any window (panel, frame,
    /// scrolled window, …).
    pub fn new_child<W: Window>(parent: &W) -> Self {
        #[cfg(target_os = "windows")]
        register_panel_class();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent = crate::platform::window_handle_as_hwnd(parent);
            let wide_class = to_wide("RuWxPanel");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN,
                0,
                0,
                100,
                100,
                parent,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(target_os = "macos")]
        let parent_handle = crate::platform::window_handle(parent);
        #[cfg(target_os = "macos")]
        let stub_handle =
            crate::platform::appkit::create_panel(parent_handle, Rect::new(0, 0, 100, 100));
        #[cfg(target_os = "linux")]
        let parent_handle = crate::platform::window_handle(parent);
        #[cfg(target_os = "linux")]
        let stub_handle =
            crate::platform::gtk::create_panel(parent_handle, Rect::new(0, 0, 100, 100));

        let panel = Panel {
            inner: Rc::new(RefCell::new(PanelInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(all(
                    not(target_os = "windows"),
                    not(target_os = "macos"),
                    not(target_os = "linux")
                ))]
                stub_handle: crate::platform::stub_backend::alloc_widget_handle(),
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                stub_handle,
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                parent_handle,
                rect: Rect::new(0, 0, 100, 100),
                widgets: Vec::new(),
                background_colour: Colour::LIGHT_GREY,
                enabled: true,
                visible: true,
                sizer: None,
            })),
        };

        #[cfg(target_os = "windows")]
        {
            let inner_clone = panel.inner.clone();
            let raw = Rc::into_raw(inner_clone);
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw as isize);
            }
        }

        #[cfg(target_os = "macos")]
        register_panel_macos(stub_handle, panel.inner.clone());

        panel
    }

    /// Create a new panel as a child of another panel (nested panels).
    pub fn new_in_panel(parent_panel: &Panel) -> Self {
        Self::new_child(parent_panel)
    }

    /// Add a widget to this panel
    pub fn add_widget(&self, widget: WidgetRef) {
        self.inner.borrow_mut().widgets.push(widget);
    }

    /// Install a sizer that will lay out this panel's children.
    ///
    /// The sizer is immediately laid out with the panel's *current* rect
    /// (0, 0, panel-width, panel-height). It will be re-laid out
    /// automatically every time the panel is resized via
    /// `Widget::set_size` — this is what allows tab pages to reflow when
    /// the tab control is resized by the frame's sizer.
    ///
    /// Typical usage:
    /// 1. Create the panel.
    /// 2. Create child widgets with this panel as their parent.
    /// 3. Build a `BoxSizer`, add the widgets to it.
    /// 4. Call `panel.set_sizer(sizer)`.
    pub fn set_sizer(&self, sizer: BoxSizer) {
        let (w, h) = {
            let inner = self.inner.borrow();
            (inner.rect.width, inner.rect.height)
        };
        let mut sizer = sizer;
        sizer.layout(0, 0, w, h);
        self.inner.borrow_mut().sizer = Some(sizer);
    }

    /// Set the background colour of the panel
    pub fn set_background_colour(&self, colour: Colour) {
        self.inner.borrow_mut().background_colour = colour;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            InvalidateRect(hwnd, std::ptr::null(), 1);
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::set_container_background(
                self.inner.borrow().stub_handle,
                colour,
            );
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_container_background(self.inner.borrow().stub_handle, colour);
        }
    }

    /// Get a WidgetRef for use with sizers
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }

    /// Show the panel (and its child widgets).
    ///
    /// Book / Tab containers call this on the page panel that is being
    /// activated and [`Panel::hide`] on all sibling pages.
    pub fn show(&self) {
        self.inner.borrow_mut().visible = true;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            ShowWindow(self.inner.borrow().hwnd, SW_SHOW);
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::set_container_visible(self.inner.borrow().stub_handle, true);
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_view_visible(self.inner.borrow().stub_handle, true);
        }
    }

    /// Hide the panel (and its child widgets).
    ///
    /// Book / Tab containers call this on the page panel that is being
    /// deactivated and [`Panel::show`] on the new active page.
    pub fn hide(&self) {
        self.inner.borrow_mut().visible = false;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            ShowWindow(self.inner.borrow().hwnd, SW_HIDE);
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::set_container_visible(self.inner.borrow().stub_handle, false);
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_view_visible(self.inner.borrow().stub_handle, false);
        }
    }

    /// Return the panel's current visibility state.
    pub fn is_visible(&self) -> bool {
        self.inner.borrow().visible
    }

    /// Return the panel's native window handle.
    pub fn hwnd(&self) -> crate::platform::NativeHandle {
        #[cfg(target_os = "windows")]
        {
            self.inner.borrow().hwnd as crate::platform::NativeHandle
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.inner.borrow().stub_handle
        }
    }

    /// Move the panel to the given absolute position (in the parent's
    /// coordinate system). Also triggers a re-layout of the panel's
    /// sizer (if any) so child widgets follow along.
    pub fn set_position(&self, x: i32, y: i32) {
        // Release the RefCell borrow BEFORE calling MoveWindow, because
        // MoveWindow can synchronously trigger WM_ERASEBKGND (and other
        // paint-time messages) on the panel, which re-enters the WndProc
        // and tries to borrow the same RefCell.
        #[cfg(target_os = "windows")]
        let hwnd = {
            let mut inner = self.inner.borrow_mut();
            inner.rect.x = x;
            inner.rect.y = y;
            inner.hwnd
        };
        #[cfg(target_os = "windows")]
        {
            let inner = self.inner.borrow();
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                MoveWindow(
                    hwnd,
                    x,
                    y,
                    inner.rect.width as i32,
                    inner.rect.height as i32,
                    1,
                );
            }
        }
        #[cfg(target_os = "macos")]
        {
            let (parent, handle) = {
                let mut inner = self.inner.borrow_mut();
                inner.rect.x = x;
                inner.rect.y = y;
                (inner.parent_handle, inner.stub_handle)
            };
            let rect = self.inner.borrow().rect;
            crate::platform::appkit::set_container_frame(parent, handle, rect);
        }
        #[cfg(target_os = "linux")]
        {
            let (parent, handle) = {
                let mut inner = self.inner.borrow_mut();
                inner.rect.x = x;
                inner.rect.y = y;
                (inner.parent_handle, inner.stub_handle)
            };
            let rect = self.inner.borrow().rect;
            crate::platform::gtk::set_view_frame(parent, handle, rect);
        }
    }

    /// Resize the panel. Also triggers a re-layout of the panel's
    /// sizer (if any) so child widgets follow along.
    pub fn set_size(&self, w: u32, h: u32) {
        // Release the RefCell borrow BEFORE calling MoveWindow, because
        // MoveWindow can synchronously trigger WM_ERASEBKGND (and other
        // paint-time messages) on the panel, which re-enters the WndProc
        // and tries to borrow the same RefCell.
        #[cfg(target_os = "windows")]
        let (hwnd, x, y) = {
            let mut inner = self.inner.borrow_mut();
            inner.rect.width = w;
            inner.rect.height = h;
            (inner.hwnd, inner.rect.x, inner.rect.y)
        };
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(hwnd, x, y, w as i32, h as i32, 1);
        }
        #[cfg(target_os = "macos")]
        let (parent, handle, rect) = {
            let mut inner = self.inner.borrow_mut();
            inner.rect.width = w;
            inner.rect.height = h;
            (inner.parent_handle, inner.stub_handle, inner.rect)
        };
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::set_container_frame(parent, handle, rect);
        }
        #[cfg(target_os = "linux")]
        let (parent, handle, rect) = {
            let mut inner = self.inner.borrow_mut();
            inner.rect.width = w;
            inner.rect.height = h;
            (inner.parent_handle, inner.stub_handle, inner.rect)
        };
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_view_frame(parent, handle, rect);
        }
        // Re-layout the sizer (if any) with the new dimensions. The
        // sizer is taken out of the RefCell, the borrow is released,
        // and the sizer is laid out. This is what allows tab pages to
        // reflow when the tab control is resized. Child widgets are
        // resized via MoveWindow, which is safe because each child has
        // its own RefCell.
        let mut sizer = self.inner.borrow_mut().sizer.take();
        if let Some(ref mut sizer) = sizer {
            sizer.layout(0, 0, w, h);
        }
        self.inner.borrow_mut().sizer = sizer;
    }
}

impl Widget for PanelInner {
    fn native_handle(&self) -> isize {
        #[cfg(target_os = "windows")]
        {
            self.hwnd as isize
        }

        #[cfg(not(target_os = "windows"))]
        {
            self.stub_handle
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
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::set_container_frame(
                self.parent_handle,
                self.stub_handle,
                self.rect,
            );
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_view_frame(self.parent_handle, self.stub_handle, self.rect);
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
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::set_container_frame(
                self.parent_handle,
                self.stub_handle,
                self.rect,
            );
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_view_frame(self.parent_handle, self.stub_handle, self.rect);
        }

        // Re-layout the sizer (if any) with the new dimensions. This is
        // what allows tab pages to reflow when the tab control is
        // resized.
        //
        // We *temporarily take the sizer out* of `self` while laying it
        // out, so the RefCell borrow is released before `MoveWindow` is
        // invoked on the child widgets. Without this, the synchronous
        // re-paint messages that Win32 emits during `MoveWindow` would
        // re-enter this same RefCell and panic.
        //
        // We pass `(0, 0, w, h)` (not the panel's own position) because
        // the sizer is laying out *the panel's child widgets* in the
        // panel's own client-area coordinate system — they are children
        // of the panel HWND, not of the panel's parent.
        let mut sizer = self.sizer.take();
        if let Some(ref mut sizer) = sizer {
            sizer.layout(0, 0, w, h);
        }
        self.sizer = sizer;
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
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::set_container_visible(self.stub_handle, visible);
        }
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_view_visible(self.stub_handle, visible);
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
        #[cfg(target_os = "linux")]
        {
            crate::platform::gtk::set_view_enabled(self.stub_handle, enabled);
        }
    }
}

// ── macOS tab page layout ───────────────────────────────────────────

#[cfg(target_os = "macos")]
thread_local! {
    static PANEL_REGISTRY: RefCell<std::collections::HashMap<
        crate::platform::NativeHandle,
        Rc<RefCell<PanelInner>>,
    >> = RefCell::new(std::collections::HashMap::new());
}

#[cfg(target_os = "macos")]
fn register_panel_macos(handle: crate::platform::NativeHandle, inner: Rc<RefCell<PanelInner>>) {
    PANEL_REGISTRY.with(|map| {
        map.borrow_mut().insert(handle, inner);
    });
}

/// Resize a tab page panel to the NSTabView content area and re-run its sizer.
#[cfg(target_os = "macos")]
pub fn relayout_tab_page(
    panel_handle: crate::platform::NativeHandle,
    tab_handle: crate::platform::NativeHandle,
    width: u32,
    height: u32,
) {
    PANEL_REGISTRY.with(|map| {
        let Some(inner_rc) = map.borrow().get(&panel_handle).cloned() else {
            return;
        };
        let mut inner = match inner_rc.try_borrow_mut() {
            Ok(inner) => inner,
            Err(_) => {
                schedule_tab_page_relayout(panel_handle, tab_handle, width, height);
                return;
            }
        };
        relayout_tab_page_inner(&mut inner, tab_handle, width, height);
    });
}

#[cfg(target_os = "macos")]
fn relayout_tab_page_inner(
    inner: &mut PanelInner,
    tab_handle: crate::platform::NativeHandle,
    width: u32,
    height: u32,
) {
    inner.parent_handle = tab_handle;
    inner.rect.x = 0;
    inner.rect.y = 0;
    inner.rect.width = width;
    inner.rect.height = height;
    let mut sizer = inner.sizer.take();
    if let Some(ref mut sizer) = sizer {
        sizer.layout(0, 0, width, height);
    }
    inner.sizer = sizer;
}

#[cfg(target_os = "macos")]
thread_local! {
    static PENDING_TAB_PAGE_LAYOUTS: RefCell<
        Vec<(crate::platform::NativeHandle, crate::platform::NativeHandle, u32, u32)>,
    > = const { RefCell::new(Vec::new()) };
    static TAB_PAGE_LAYOUT_SCHEDULED: Cell<bool> = const { Cell::new(false) };
}

#[cfg(target_os = "macos")]
fn try_relayout_tab_page(
    inner_rc: &Rc<RefCell<PanelInner>>,
    tab_handle: crate::platform::NativeHandle,
    width: u32,
    height: u32,
) {
    if let Ok(mut inner) = inner_rc.try_borrow_mut() {
        relayout_tab_page_inner(&mut inner, tab_handle, width, height);
    }
}

#[cfg(target_os = "macos")]
fn schedule_tab_page_relayout(
    panel_handle: crate::platform::NativeHandle,
    tab_handle: crate::platform::NativeHandle,
    width: u32,
    height: u32,
) {
    PENDING_TAB_PAGE_LAYOUTS.with(|q| {
        q.borrow_mut()
            .push((panel_handle, tab_handle, width, height));
    });
    if TAB_PAGE_LAYOUT_SCHEDULED.get() {
        return;
    }
    TAB_PAGE_LAYOUT_SCHEDULED.set(true);
    crate::platform::appkit::run_on_next_event(|| {
        TAB_PAGE_LAYOUT_SCHEDULED.set(false);
        let pending: Vec<_> =
            PENDING_TAB_PAGE_LAYOUTS.with(|q| std::mem::take(&mut *q.borrow_mut()));
        for (panel_handle, tab_handle, width, height) in pending {
            PANEL_REGISTRY.with(|map| {
                let Some(inner_rc) = map.borrow().get(&panel_handle).cloned() else {
                    return;
                };
                try_relayout_tab_page(&inner_rc, tab_handle, width, height);
            });
        }
        crate::platform::appkit::flush_scrolled_resize_pending();
    });
}

impl Window for Panel {
    fn hwnd(&self) -> crate::platform::NativeHandle {
        Panel::hwnd(self)
    }
}

/// Win32 Window Procedure for the panel class
///
/// Panels are typically used as containers for child controls. Any
/// unhandled Win32 message (including `WM_COMMAND` from a child Button,
/// CheckBox, etc. and `WM_NOTIFY` from a child ListCtrl or other common
/// control) is forwarded to the *parent* window via `SendMessageW`, so
/// the parent's message dispatch can find the right control handler.
///
/// This forwarding is essential for the tabbed-interface use case: the
/// page panels are direct children of the frame (not of the tab
/// control), so child controls of a page panel need their messages
/// delivered to the frame's `WM_COMMAND`/`WM_NOTIFY` dispatcher.
#[cfg(target_os = "windows")]
unsafe extern "system" fn panel_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_ERASEBKGND => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                // The build code stored the panel's
                // `Rc<RefCell<PanelInner>>` in `GWLP_USERDATA` via
                // `Rc::into_raw(self.inner.clone())`, which leaves
                // the strong count at 2 (the outer `Panel` still
                // owns 1, and the "leaked" reference owns 1).
                //
                // `Rc::from_raw` does NOT increment the strong
                // count on its own — it just reconstructs an `Rc`
                // claiming the leaked slot. We need to bump the
                // count first, otherwise the matching `drop(rc)`
                // at the end of the arm would put the count at 1
                // on the first dispatch, at 0 (and deallocate the
                // backing storage) on the second, and every
                // subsequent dispatch would be a use-after-free.
                // `WM_ERASEBKGND` fires on every repaint, so this
                // bug bites very quickly.
                unsafe {
                    Rc::increment_strong_count(ptr as *const RefCell<PanelInner>);
                }
                let rc = unsafe { Rc::from_raw(ptr as *const RefCell<PanelInner>) };
                // `try_borrow`: questo wndproc può essere rientrato
                // sincronamente mentre `sizer.layout` (chiamato da
                // `set_size` con `&mut self`, quindi `borrow_mut` attivo
                // sullo stesso `RefCell`) esegue `MoveWindow` sui figli.
                // In quel caso il colore di sfondo non è leggibile:
                // dichiariamo l'erase fatto e lasciamo che sia il
                // prossimo ciclo di paint a sistemare, invece di
                // andare in panic ("RefCell already mutably borrowed",
                // es. `music_player`).
                let colour = rc.try_borrow().map(|inner| inner.background_colour);
                drop(rc); // Release Rc before any Win32 painting calls
                let Ok(colour) = colour else {
                    return 1;
                };

                let hdc = wparam as HDC;
                let mut rect: RECT = std::mem::zeroed();
                GetClientRect(hwnd, &mut rect);
                let brush = CreateSolidBrush(colour.to_colorref());
                FillRect(hdc, &rect, brush);
                DeleteObject(brush);
                return 1;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_DESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let _ = Rc::from_raw(ptr as *const RefCell<PanelInner>);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        // Forward `WM_COMMAND`, `WM_NOTIFY`, and scroll notifications
        // to the parent. These are the messages the parent frame needs
        // to receive from a child control that lives on a panel, so
        // the frame's command / notify / scroll dispatcher can find the
        // right control handler.
        //
        // We deliberately do NOT forward every unhandled message. In
        // particular, broadcast messages such as `WM_UPDATEUISTATE`
        // (`0x0128`) are normally handled by `DefWindowProcW`, which
        // then *rebroadcasts* the message to every child window.
        // Forwarding that up to the parent and back creates an infinite
        // message loop that hangs the thread inside `CreateWindowExW`
        // when a control is being created on a panel. The same hazard
        // exists for `WM_SETCURSOR`, `WM_NCHITTEST`, etc.
        WM_COMMAND | WM_NOTIFY | WM_HSCROLL | WM_VSCROLL => {
            let parent = GetParent(hwnd);
            if !parent.is_null() {
                SendMessageW(parent, msg, wparam, lparam)
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
        // Resize esterno del pannello (es. `MoveWindow` eseguito dal
        // pane host dell'AuiManager): ri-applichiamo il sizer con le
        // nuove dimensioni client, così i figli (bottoni, label…) non
        // restano con la geometria del vecchio rect (caso "Actions"
        // con bottoni h=200 in una fascia da 45 px).
        WM_SIZE => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                unsafe {
                    Rc::increment_strong_count(ptr as *const RefCell<PanelInner>);
                }
                let rc = unsafe { Rc::from_raw(ptr as *const RefCell<PanelInner>) };
                let mut client: RECT = unsafe { std::mem::zeroed() };
                unsafe { GetClientRect(hwnd, &mut client) };
                let (w, h) = (client.right as u32, client.bottom as u32);
                // `try_borrow_mut`: questo wndproc può essere rientrato
                // sincronamente mentre `set_size` (che ha già `&mut
                // PanelInner`) esegue `MoveWindow`; in quel caso il
                // ri-layout lo fa già `set_size` stesso.
                if let Ok(mut inner) = rc.try_borrow_mut() {
                    inner.rect.width = w;
                    inner.rect.height = h;
                    let mut sizer = inner.sizer.take();
                    if let Some(ref mut s) = sizer {
                        s.layout(0, 0, w, h);
                    }
                    inner.sizer = sizer;
                }
                drop(rc);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
