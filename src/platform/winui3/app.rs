//! WinUI3 Application and Widget Types
//!
//! This module provides the core WinUI3 application types and widget
//! implementations using pure Win32 native controls.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ptr;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::dpi;
use super::events;
use super::layout::BoxSizer;
use super::menu::MenuBar;
use super::widgets::WidgetParent;

// ============================================================================
// Per-frame callback registry for WM_SIZE, WM_KEYDOWN, WM_MOUSE* events.
// The wnd_proc reads from this map to dispatch to the correct closure.
// ============================================================================

type ResizeCallback = Box<dyn FnMut(u32, u32)>;
type KeyCallback = Box<dyn FnMut(u8)>;
type MouseCallback = Box<dyn FnMut(i32, i32)>;
type WheelCallback = Box<dyn FnMut(i32)>;

#[derive(Default)]
struct FrameCallbacks {
    on_resize: Option<ResizeCallback>,
    on_key_down: Option<KeyCallback>,
    on_key_up: Option<KeyCallback>,
    on_mouse: Option<MouseCallback>,
    on_mouse_wheel: Option<WheelCallback>,
}

thread_local! {
    static FRAME_CALLBACKS: RefCell<HashMap<isize, FrameCallbacks>> = RefCell::new(HashMap::new());
}

// ============================================================================
// Utility
// ============================================================================

/// Encode a Rust `&str` as a null-terminated UTF-16 buffer.
pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

thread_local! {
    static ID_COUNTER: RefCell<u16> = const { RefCell::new(1000) };
    static CLASS_REGISTERED: RefCell<bool> = const { RefCell::new(false) };
}

/// Get the next available control id.
pub fn next_id() -> u16 {
    ID_COUNTER.with(|counter| {
        let mut id = counter.borrow_mut();
        *id += 1;
        *id
    })
}

const WS_CHILD_VISIBLE: u32 = WS_CHILD | WS_VISIBLE;

// ============================================================================
// App
// ============================================================================

pub struct App;

impl App {
    pub fn new() -> Self {
        ensure_class_registered();
        App
    }
    pub fn run<F>(&self, _frame: &Frame, _on_idle: F)
    where
        F: Fn() + 'static,
    {
        self.run_with_callback(|_, _, _, _| None);
    }
    pub fn run_with_callback<F>(&self, _callback: F)
    where
        F: FnMut(HWND, u32, usize, isize) -> Option<isize> + 'static,
    {
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}
impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> isize {
    match msg {
        WM_DESTROY => {
            // Clean up per-frame callbacks.
            FRAME_CALLBACKS.with(|m| m.borrow_mut().remove(&(hwnd as isize)));
            PostQuitMessage(0);
            0
        }
        WM_SIZE => {
            let width = (lparam & 0xFFFF) as i32;
            let height = ((lparam >> 16) & 0xFFFF) as i32;
            FRAME_CALLBACKS.with(|map| {
                let mut map = map.borrow_mut();
                if let Some(entry) = map.get_mut(&(hwnd as isize)) {
                    if let Some(ref mut cb) = entry.on_resize {
                        cb(width as u32, height as u32);
                    }
                }
            });
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_KEYDOWN => {
            let vk = (wparam & 0xFF) as u8;
            FRAME_CALLBACKS.with(|map| {
                let mut map = map.borrow_mut();
                if let Some(entry) = map.get_mut(&(hwnd as isize)) {
                    if let Some(ref mut cb) = entry.on_key_down {
                        cb(vk);
                    }
                }
            });
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN => {
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            FRAME_CALLBACKS.with(|map| {
                let mut map = map.borrow_mut();
                if let Some(entry) = map.get_mut(&(hwnd as isize)) {
                    if let Some(ref mut cb) = entry.on_mouse {
                        cb(x, y);
                    }
                }
            });
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_KEYUP => {
            let vk = (wparam & 0xFF) as u8;
            FRAME_CALLBACKS.with(|map| {
                let mut map = map.borrow_mut();
                if let Some(entry) = map.get_mut(&(hwnd as isize)) {
                    if let Some(ref mut cb) = entry.on_key_up {
                        cb(vk);
                    }
                }
            });
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_MOUSEWHEEL => {
            // wparam_hi = wheel delta (positive = up, negative = down)
            let delta = ((wparam >> 16) & 0xFFFF) as i16 as i32;
            FRAME_CALLBACKS.with(|map| {
                let mut map = map.borrow_mut();
                if let Some(entry) = map.get_mut(&(hwnd as isize)) {
                    if let Some(ref mut cb) = entry.on_mouse_wheel {
                        cb(delta);
                    }
                }
            });
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_COMMAND => DefWindowProcW(hwnd, msg, wparam, lparam),
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub fn ensure_class_registered() {
    let already = CLASS_REGISTERED.with(|c| *c.borrow());
    if already {
        return;
    }
    #[repr(C)]
    #[allow(non_snake_case)] // field names are pinned to the INITCOMMONCONTROLSEX ABI
    struct InitCcx {
        dwSize: u32,
        dwICC: u32,
    }
    unsafe {
        let icc = InitCcx {
            dwSize: std::mem::size_of::<InitCcx>() as u32,
            dwICC: 0xFFFF,
        };
        extern "system" {
            fn InitCommonControlsEx(picce: *const InitCcx) -> i32;
        }
        let icc_ok = InitCommonControlsEx(&icc);
        // Senza manifest v6 nel binario Win32 carica comctl32 v5, che
        // rifiuta i flag moderni (ICC_LINK_CLASS, ICC_DATE_CLASSES, …):
        // stessa rete di sicurezza di `core::app::App::new` — attiva un
        // contesto ACTCTX runtime con le Common-Controls 6.0.0.0 e
        // riprove, così i controlli aprono sempre con i visual styles
        // Windows 11 anche su questo backend.
        if icc_ok == 0 && crate::core::app::activate_common_controls_v6() {
            let _ = InitCommonControlsEx(&icc);
        }
    }
    unsafe {
        let class_name = to_wide("ru_wx_Frame");
        let hinstance = GetModuleHandleW(ptr::null());
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: ptr::null_mut(),
            hCursor: ptr::null_mut(),
            hbrBackground: COLOR_WINDOW as HBRUSH,
            lpszMenuName: ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: ptr::null_mut(),
        };
        let _ = RegisterClassExW(&wc);
    }
    CLASS_REGISTERED.with(|c| *c.borrow_mut() = true);
}

// ============================================================================
// Frame
// ============================================================================

pub struct Frame {
    hwnd: HWND,
    title: String,
    width: i32,
    height: i32,
    sizer: Option<BoxSizer>,
    menu_bar: Option<MenuBar>,
}

impl Frame {
    pub fn new(title: &str, width: i32, height: i32) -> Self {
        ensure_class_registered();
        unsafe {
            let hwnd = CreateWindowExW(
                0,
                to_wide("ru_wx_Frame").as_ptr(),
                to_wide(title).as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                100,
                100,
                width,
                height,
                ptr::null_mut(),
                ptr::null_mut(),
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            );
            Frame {
                hwnd,
                title: title.to_string(),
                width,
                height,
                sizer: None,
                menu_bar: None,
            }
        }
    }
    pub fn builder() -> FrameBuilder {
        FrameBuilder::new()
    }
    pub fn as_widget_ref(&self) -> crate::core::widget::WidgetRef {
        super::window_trait::make_widget_ref(self.hwnd)
    }
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
    pub fn hwnd_native(&self) -> isize {
        self.hwnd as isize
    }
    pub fn set_title(&self, title: &str) {
        unsafe {
            SetWindowTextW(self.hwnd, to_wide(title).as_ptr());
        }
    }
    pub fn get_title(&self) -> String {
        self.title.clone()
    }
    pub fn set_size(&self, w: i32, h: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                0,
                0,
                w,
                h,
                SWP_NOZORDER | SWP_NOMOVE,
            );
        }
    }
    pub fn get_size(&self) -> (i32, i32) {
        unsafe {
            let mut r: RECT = std::mem::zeroed();
            GetWindowRect(self.hwnd, &mut r);
            (r.right - r.left, r.bottom - r.top)
        }
    }
    pub fn center(&self) {
        unsafe {
            let mut r: RECT = std::mem::zeroed();
            GetWindowRect(self.hwnd, &mut r);
            let sw = GetSystemMetrics(SM_CXSCREEN);
            let sh = GetSystemMetrics(SM_CYSCREEN);
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                (sw - (r.right - r.left)) / 2,
                (sh - (r.bottom - r.top)) / 2,
                0,
                0,
                SWP_NOZORDER | SWP_NOSIZE,
            );
        }
    }
    pub fn show(&self) {
        unsafe {
            ShowWindow(self.hwnd, SW_SHOW);
        }
    }
    pub fn hide(&self) {
        unsafe {
            ShowWindow(self.hwnd, SW_HIDE);
        }
    }
    pub fn close(&self) {
        unsafe {
            PostMessageW(self.hwnd, WM_CLOSE, 0, 0);
        }
    }
    pub fn set_on_close<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        events::register_close(self.hwnd, callback);
    }
    pub fn on_command<F: FnMut(events::WidgetEvent) + 'static>(&self, id: u16, f: F) {
        events::register_command(self.hwnd, id, f);
    }
    pub fn client_size(&self) -> (i32, i32) {
        unsafe {
            let mut r: RECT = std::mem::zeroed();
            GetClientRect(self.hwnd, &mut r);
            (r.right - r.left, r.bottom - r.top)
        }
    }

    /// Query the system DPI of this frame's HWND.
    pub fn dpi(&self) -> u32 {
        dpi::query(self.hwnd)
    }

    /// Returns the DPI scale factor (1.0 at 96 DPI, 2.0 at 192 DPI).
    pub fn scale_factor(&self) -> f64 {
        self.dpi() as f64 / 96.0
    }

    /// Register a handler invoked whenever the frame is resized.
    /// The callback receives the new `(width, height)` of the client area.
    pub fn on_resize<F: FnMut(u32, u32) + 'static>(&self, f: F) {
        FRAME_CALLBACKS.with(|map| {
            let mut map = map.borrow_mut();
            let entry = map.entry(self.hwnd as isize).or_default();
            entry.on_resize = Some(Box::new(f));
        });
    }

    /// Register a handler invoked on every `WM_KEYDOWN`.
    /// The callback receives the virtual-key code.
    pub fn on_key_down<F: FnMut(u8) + 'static>(&self, f: F) {
        FRAME_CALLBACKS.with(|map| {
            let mut map = map.borrow_mut();
            let entry = map.entry(self.hwnd as isize).or_default();
            entry.on_key_down = Some(Box::new(f));
        });
    }

    /// Register a handler invoked on mouse down (left button).
    /// The callback receives the client-area `(x, y)`.
    pub fn on_mouse<F: FnMut(i32, i32) + 'static>(&self, f: F) {
        FRAME_CALLBACKS.with(|map| {
            let mut map = map.borrow_mut();
            let entry = map.entry(self.hwnd as isize).or_default();
            entry.on_mouse = Some(Box::new(f));
        });
    }

    /// Register a handler invoked on `WM_KEYUP`.
    /// The callback receives the virtual-key code.
    pub fn on_key_up<F: FnMut(u8) + 'static>(&self, f: F) {
        FRAME_CALLBACKS.with(|map| {
            let mut map = map.borrow_mut();
            let entry = map.entry(self.hwnd as isize).or_default();
            entry.on_key_up = Some(Box::new(f));
        });
    }

    /// Register a handler invoked on `WM_MOUSEWHEEL`.
    /// The callback receives the wheel delta (positive = up, negative = down).
    pub fn on_mouse_wheel<F: FnMut(i32) + 'static>(&self, f: F) {
        FRAME_CALLBACKS.with(|map| {
            let mut map = map.borrow_mut();
            let entry = map.entry(self.hwnd as isize).or_default();
            entry.on_mouse_wheel = Some(Box::new(f));
        });
    }

    /// Register a handler invoked when the user requests a close
    /// (WM_CLOSE). Alias for `set_on_close`.
    pub fn on_query_close<F: Fn() + 'static>(&self, f: F) {
        self.set_on_close(f);
    }

    /// Set the frame icon from an SVG byte slice.
    pub fn set_icon_from_svg(&self, svg_data: &[u8], size: u32) {
        if let Some(h) = super::icon::svg_to_hicon(svg_data, size) {
            super::icon::set_frame_icon(self.hwnd, h);
        }
    }

    /// Get the screen position of the frame (outer rect, including title bar).
    pub fn get_position(&self) -> (i32, i32) {
        unsafe {
            let mut r: RECT = std::mem::zeroed();
            GetWindowRect(self.hwnd, &mut r);
            (r.left, r.top)
        }
    }

    /// Move the frame to the given screen position.
    pub fn set_position(&self, x: i32, y: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOZORDER | SWP_NOSIZE,
            );
        }
    }

    /// Set the minimum client size hint (used by sizers during layout).
    pub fn set_min_client_size(&self, w: i32, h: i32) {
        unsafe {
            let mut r: RECT = std::mem::zeroed();
            GetWindowRect(self.hwnd, &mut r);
            let border = r.right - r.left - self.width;
            let title = r.bottom - r.top - self.height;
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                0,
                0,
                w + border,
                h + title,
                SWP_NOZORDER | SWP_NOMOVE,
            );
        }
    }

    #[allow(clippy::should_implement_trait)] // inherent handle-clone by design; not the Clone trait
    pub fn clone(&self) -> Frame {
        Frame {
            hwnd: self.hwnd,
            title: self.title.clone(),
            width: self.width,
            height: self.height,
            sizer: None,
            menu_bar: None,
        }
    }
    pub fn set_sizer(&mut self, sizer: BoxSizer) {
        self.sizer = Some(sizer);
        if let Some(ref s) = self.sizer {
            s.layout(self.hwnd);
        }
    }
    pub fn set_menu_bar(&mut self, menu_bar: MenuBar) {
        unsafe {
            SetMenu(self.hwnd, menu_bar.hmenu());
        }
        self.menu_bar = Some(menu_bar);
    }
    pub fn set_size_hints(&self) {}
}
impl WidgetParent for Frame {
    fn hwnd(&self) -> HWND {
        self.hwnd
    }
}
impl Drop for Frame {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

pub struct FrameBuilder {
    title: String,
    width: i32,
    height: i32,
    x: i32,
    y: i32,
}
impl FrameBuilder {
    pub fn new() -> Self {
        Self {
            title: "ru_wx Window".into(),
            width: 800,
            height: 600,
            x: 100,
            y: 100,
        }
    }
    pub fn with_title(mut self, t: &str) -> Self {
        self.title = t.into();
        self
    }
    pub fn with_size(mut self, w: i32, h: i32) -> Self {
        self.width = w;
        self.height = h;
        self
    }
    pub fn with_position(mut self, x: i32, y: i32) -> Self {
        self.x = x;
        self.y = y;
        self
    }
    pub fn build(self) -> Frame {
        ensure_class_registered();
        unsafe {
            let hwnd = CreateWindowExW(
                0,
                to_wide("ru_wx_Frame").as_ptr(),
                to_wide(&self.title).as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                self.x,
                self.y,
                self.width,
                self.height,
                ptr::null_mut(),
                ptr::null_mut(),
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            );
            Frame {
                hwnd,
                title: self.title,
                width: self.width,
                height: self.height,
                sizer: None,
                menu_bar: None,
            }
        }
    }
}
impl Default for FrameBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Button
// ============================================================================

/// Event payload for a button click.
#[derive(Debug, Clone)]
pub struct ButtonEvent {
    pub id: u16,
    pub label: String,
}

/// Event payload for a checkbox toggle.
#[derive(Debug, Clone)]
pub struct CheckBoxEvent {
    pub id: u16,
    pub checked: bool,
}

pub struct Button {
    hwnd: HWND,
    id: u16,
    label: String,
}

impl Button {
    pub fn new<P: WidgetParent>(parent: &P, label: &str) -> Self {
        let id = next_id();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide("BUTTON").as_ptr(),
                to_wide(label).as_ptr(),
                WS_CHILD_VISIBLE | 0x0000_0001,
                0,
                0,
                100,
                30,
                parent.hwnd(),
                id as isize as *mut std::ffi::c_void,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            )
        };
        Self {
            hwnd,
            id,
            label: label.into(),
        }
    }
    pub fn set_label(&self, label: &str) {
        unsafe {
            SetWindowTextW(self.hwnd, to_wide(label).as_ptr());
        }
    }
    pub fn get_label(&self) -> String {
        self.label.clone()
    }
    pub fn id(&self) -> u16 {
        self.id
    }
    pub fn set_size(&self, w: i32, h: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                0,
                0,
                w,
                h,
                SWP_NOZORDER | SWP_NOMOVE,
            );
        }
    }
    pub fn set_position(&self, x: i32, y: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOZORDER | SWP_NOSIZE,
            );
        }
    }
    pub fn on_click<F>(&self, parent: &Frame, callback: F)
    where
        F: Fn() + 'static,
    {
        events::register_command(parent.hwnd(), self.id, move |_| {
            (callback)();
        });
    }

    /// Register a handler that receives a `ButtonEvent` with metadata.
    pub fn on_click_event<F>(&self, parent: &Frame, callback: F)
    where
        F: Fn(ButtonEvent) + 'static,
    {
        let id = self.id;
        let label = self.label.clone();
        events::register_command(parent.hwnd(), id, move |_| {
            (callback)(ButtonEvent {
                id,
                label: label.clone(),
            });
        });
    }

    /// Create a button with an SVG icon rendered to a bitmap.
    pub fn new_with_svg_bytes<P: WidgetParent>(parent: &P, svg_data: &[u8], size: u32) -> Self {
        let id = next_id();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide("BUTTON").as_ptr(),
                to_wide("").as_ptr(),
                WS_CHILD_VISIBLE | 0x0000_0040, /* BS_ICON */
                0,
                0,
                size as i32 + 8,
                size as i32 + 8,
                parent.hwnd(),
                id as isize as *mut std::ffi::c_void,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            )
        };
        if let Some(h) = super::icon::svg_to_hicon(svg_data, size) {
            unsafe {
                SendMessageW(
                    hwnd, 0x00F7, /* BM_SETIMAGE */
                    0,      /* IMAGE_ICON */
                    h as isize,
                );
            }
        }
        Self {
            hwnd,
            id,
            label: String::new(),
        }
    }

    /// Create a button with a bitmap icon.
    pub fn new_with_bitmap<P: WidgetParent>(
        parent: &P,
        bitmap: windows_sys::Win32::Graphics::Gdi::HBITMAP,
    ) -> Self {
        let id = next_id();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide("BUTTON").as_ptr(),
                to_wide("").as_ptr(),
                WS_CHILD_VISIBLE | 0x0000_0040, /* BS_ICON */
                0,
                0,
                40,
                40,
                parent.hwnd(),
                id as isize as *mut std::ffi::c_void,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            )
        };
        unsafe {
            SendMessageW(hwnd, 0x00F7, 1 /* IMAGE_BITMAP */, bitmap as isize);
        }
        Self {
            hwnd,
            id,
            label: String::new(),
        }
    }

    pub fn as_widget_ref(&self) -> crate::core::widget::WidgetRef {
        super::window_trait::make_widget_ref(self.hwnd)
    }
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}
impl Drop for Button {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

// ============================================================================
// StaticText
// ============================================================================

pub struct StaticText {
    hwnd: HWND,
    text: String,
}

impl StaticText {
    pub fn new<P: WidgetParent>(parent: &P, text: &str) -> Self {
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide("STATIC").as_ptr(),
                to_wide(text).as_ptr(),
                WS_CHILD_VISIBLE,
                0,
                0,
                200,
                20,
                parent.hwnd(),
                ptr::null_mut(),
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            )
        };
        Self {
            hwnd,
            text: text.into(),
        }
    }
    pub fn set_label(&self, t: &str) {
        unsafe {
            SetWindowTextW(self.hwnd, to_wide(t).as_ptr());
        }
    }
    pub fn get_label(&self) -> String {
        self.text.clone()
    }
    pub fn set_size(&self, w: i32, h: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                0,
                0,
                w,
                h,
                SWP_NOZORDER | SWP_NOMOVE,
            );
        }
    }
    pub fn set_position(&self, x: i32, y: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOZORDER | SWP_NOSIZE,
            );
        }
    }
    pub fn as_widget_ref(&self) -> crate::core::widget::WidgetRef {
        super::window_trait::make_widget_ref(self.hwnd)
    }
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}
impl Drop for StaticText {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

// ============================================================================
// TextCtrl
// ============================================================================

pub struct TextCtrl {
    hwnd: HWND,
    id: u16,
    text: String,
}

impl TextCtrl {
    pub fn new<P: WidgetParent>(parent: &P, text: &str) -> Self {
        let id = next_id();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide("EDIT").as_ptr(),
                to_wide(text).as_ptr(),
                WS_CHILD_VISIBLE | 0x0000_0080,
                0,
                0,
                200,
                24,
                parent.hwnd(),
                id as isize as *mut std::ffi::c_void,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            )
        };
        Self {
            hwnd,
            id,
            text: text.into(),
        }
    }
    pub fn new_multiline<P: WidgetParent>(parent: &P, text: &str) -> Self {
        let id = next_id();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide("EDIT").as_ptr(),
                to_wide(text).as_ptr(),
                WS_CHILD_VISIBLE | 0x0004 | 0x0080 | 0x0040 | WS_VSCROLL,
                0,
                0,
                300,
                100,
                parent.hwnd(),
                id as isize as *mut std::ffi::c_void,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            )
        };
        Self {
            hwnd,
            id,
            text: text.into(),
        }
    }
    pub fn set_value(&self, t: &str) {
        unsafe {
            SetWindowTextW(self.hwnd, to_wide(t).as_ptr());
        }
    }
    pub fn get_value(&self) -> String {
        unsafe {
            let len = GetWindowTextLengthW(self.hwnd);
            if len <= 0 {
                return String::new();
            }
            let mut buf: Vec<u16> = vec![0; len as usize + 1];
            GetWindowTextW(self.hwnd, buf.as_mut_ptr(), len + 1);
            String::from_utf16_lossy(&buf[..len as usize])
        }
    }
    pub fn append_text(&self, text: &str) {
        unsafe {
            let current_len = GetWindowTextLengthW(self.hwnd) as usize;
            let mut wide = to_wide(text);
            SendMessageW(self.hwnd, 0x00B1, current_len, current_len as isize);
            SendMessageW(self.hwnd, 0x00C2, 1, wide.as_mut_ptr() as isize);
        }
    }
    pub fn set_size(&self, w: i32, h: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                0,
                0,
                w,
                h,
                SWP_NOZORDER | SWP_NOMOVE,
            );
        }
    }
    pub fn set_position(&self, x: i32, y: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOZORDER | SWP_NOSIZE,
            );
        }
    }
    pub fn as_widget_ref(&self) -> crate::core::widget::WidgetRef {
        super::window_trait::make_widget_ref(self.hwnd)
    }
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
    pub fn id(&self) -> u16 {
        self.id
    }

    /// Create a password (masked) text control.
    pub fn password<P: WidgetParent>(parent: &P, text: &str) -> Self {
        let id = next_id();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide("EDIT").as_ptr(),
                to_wide(text).as_ptr(),
                WS_CHILD_VISIBLE | 0x0000_0080 | 0x0000_0020, /* ES_PASSWORD */
                0,
                0,
                200,
                24,
                parent.hwnd(),
                id as isize as *mut std::ffi::c_void,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            )
        };
        Self {
            hwnd,
            id,
            text: text.into(),
        }
    }

    /// Set or clear read-only mode via `EM_SETREADONLY`.
    pub fn set_readonly(&self, readonly: bool) {
        unsafe {
            SendMessageW(
                self.hwnd,
                0x00CF, /* EM_SETREADONLY */
                readonly as usize,
                0,
            );
        }
    }

    /// Clear all text in the control.
    pub fn clear(&self) {
        unsafe {
            SetWindowTextW(self.hwnd, to_wide("").as_ptr());
        }
    }

    /// Undo the last edit operation.
    pub fn undo(&self) {
        unsafe {
            SendMessageW(self.hwnd, 0x00C7 /* EM_UNDO */, 0, 0);
        }
    }

    /// Returns `true` if an undo operation is available.
    pub fn can_undo(&self) -> bool {
        unsafe {
            SendMessageW(self.hwnd, 0x00C6 /* EM_CANUNDO */, 0, 0) != 0
        }
    }

    /// Set the maximum number of characters the user can enter.
    pub fn set_max_length(&self, max: u32) {
        unsafe {
            SendMessageW(
                self.hwnd,
                0x00C5, /* EM_SETLIMITTEXT */
                max as usize,
                0,
            );
        }
    }

    /// Get the current maximum text length.
    pub fn max_length(&self) -> u32 {
        unsafe {
            SendMessageW(self.hwnd, 0x00D5 /* EM_GETLIMITTEXT */, 0, 0) as u32
        }
    }

    pub fn on_change<F: Fn(String) + 'static>(&self, parent: &Frame, callback: F) {
        events::register_hwnd(parent.hwnd(), self.hwnd, move |ev| {
            if let events::WidgetEvent::TextChanged(text) = ev {
                (callback)(text);
            }
        });
    }
}
impl Drop for TextCtrl {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

// ============================================================================
// CheckBox
// ============================================================================

pub struct CheckBox {
    hwnd: HWND,
    id: u16,
    label: String,
}

impl CheckBox {
    pub fn new<P: WidgetParent>(parent: &P, label: &str) -> Self {
        let id = next_id();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide("BUTTON").as_ptr(),
                to_wide(label).as_ptr(),
                WS_CHILD_VISIBLE | 0x0000_0003,
                0,
                0,
                120,
                24,
                parent.hwnd(),
                id as isize as HMENU,
                GetModuleHandleW(ptr::null()) as HINSTANCE,
                ptr::null_mut(),
            )
        };
        Self {
            hwnd,
            id,
            label: label.into(),
        }
    }
    pub fn is_checked(&self) -> bool {
        unsafe { SendMessageW(self.hwnd, 0x00F0, 0, 0) != 0 }
    }
    pub fn set_checked(&self, checked: bool) {
        unsafe {
            SendMessageW(self.hwnd, 0x00F1, if checked { 1 } else { 0 }, 0);
        }
    }
    pub fn on_click<F>(&self, callback: F)
    where
        F: Fn(bool) + 'static,
    {
        let parent_hwnd = unsafe { GetParent(self.hwnd) };
        if parent_hwnd.is_null() {
            return;
        }
        let inner = self.hwnd;
        let id = self.id;
        events::register_command(parent_hwnd, id, move |_| {
            let is_checked = unsafe { SendMessageW(inner, 0x00F0, 0, 0) != 0 };
            (callback)(is_checked);
        });
    }

    /// Register a handler that receives a `CheckBoxEvent` with metadata.
    pub fn on_check_event<F>(&self, callback: F)
    where
        F: Fn(CheckBoxEvent) + 'static,
    {
        let parent_hwnd = unsafe { GetParent(self.hwnd) };
        if parent_hwnd.is_null() {
            return;
        }
        let inner = self.hwnd;
        let id = self.id;
        events::register_command(parent_hwnd, id, move |_| {
            let is_checked = unsafe { SendMessageW(inner, 0x00F0, 0, 0) != 0 };
            (callback)(CheckBoxEvent {
                id,
                checked: is_checked,
            });
        });
    }

    pub fn set_label(&self, label: &str) {
        unsafe {
            SetWindowTextW(self.hwnd, to_wide(label).as_ptr());
        }
    }
    pub fn get_label(&self) -> String {
        self.label.clone()
    }
    pub fn id(&self) -> u16 {
        self.id
    }
    pub fn as_widget_ref(&self) -> crate::core::widget::WidgetRef {
        super::window_trait::make_widget_ref(self.hwnd)
    }
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}
impl Drop for CheckBox {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

// ============================================================================
// RadioButton
// ============================================================================

pub struct RadioButton {
    hwnd: HWND,
    id: u16,
    label: String,
}

impl RadioButton {
    pub fn new<P: WidgetParent>(parent: &P, label: &str) -> Self {
        let id = next_id();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide("BUTTON").as_ptr(),
                to_wide(label).as_ptr(),
                WS_CHILD_VISIBLE | 0x0000_0004,
                0,
                0,
                120,
                24,
                parent.hwnd(),
                id as isize as HMENU,
                GetModuleHandleW(ptr::null()) as HINSTANCE,
                ptr::null_mut(),
            )
        };
        Self {
            hwnd,
            id,
            label: label.into(),
        }
    }
    pub fn is_checked(&self) -> bool {
        unsafe { SendMessageW(self.hwnd, 0x00F0, 0, 0) != 0 }
    }
    pub fn set_checked(&self, checked: bool) {
        unsafe {
            SendMessageW(self.hwnd, 0x00F1, if checked { 1 } else { 0 }, 0);
        }
    }
    pub fn set_label(&self, label: &str) {
        unsafe {
            SetWindowTextW(self.hwnd, to_wide(label).as_ptr());
        }
    }
    pub fn get_label(&self) -> String {
        self.label.clone()
    }
    pub fn id(&self) -> u16 {
        self.id
    }

    /// Register a handler invoked when the radio button is toggled.
    pub fn on_toggle<F: Fn(bool) + 'static>(&self, parent: &Frame, callback: F) {
        let inner = self.hwnd;
        let id = self.id;
        events::register_command(parent.hwnd(), id, move |_| {
            let is_checked = unsafe { SendMessageW(inner, 0x00F0, 0, 0) != 0 };
            (callback)(is_checked);
        });
    }

    pub fn as_widget_ref(&self) -> crate::core::widget::WidgetRef {
        super::window_trait::make_widget_ref(self.hwnd)
    }
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}
impl Drop for RadioButton {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

// ============================================================================
// Panel
// ============================================================================

pub struct Panel {
    hwnd: HWND,
    id: u16,
}

impl Panel {
    pub fn new<P: WidgetParent>(parent: &P) -> Self {
        let id = next_id();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide("STATIC").as_ptr(),
                to_wide("").as_ptr(),
                WS_CHILD_VISIBLE | 0x0000_000B,
                0,
                0,
                200,
                200,
                parent.hwnd(),
                id as isize as *mut std::ffi::c_void,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            )
        };
        Self { hwnd, id }
    }
    pub fn set_size(&self, w: i32, h: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                0,
                0,
                w,
                h,
                SWP_NOZORDER | SWP_NOMOVE,
            );
        }
    }
    pub fn set_position(&self, x: i32, y: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOZORDER | SWP_NOSIZE,
            );
        }
    }
    pub fn as_widget_ref(&self) -> crate::core::widget::WidgetRef {
        super::window_trait::make_widget_ref(self.hwnd)
    }
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}
impl WidgetParent for Panel {
    fn hwnd(&self) -> HWND {
        self.hwnd
    }
}
impl Drop for Panel {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

// ============================================================================
// Choice (dropdown combo)
// ============================================================================

pub struct Choice {
    hwnd: HWND,
    id: u16,
    items: Vec<String>,
    selected: Option<i32>,
}

impl Choice {
    pub fn new<P: WidgetParent>(parent: &P) -> Self {
        let id = next_id();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide("COMBOBOX").as_ptr(),
                to_wide("").as_ptr(),
                WS_CHILD_VISIBLE | 0x0003,
                0,
                0,
                150,
                200,
                parent.hwnd(),
                id as isize as *mut std::ffi::c_void,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            )
        };
        Self {
            hwnd,
            id,
            items: Vec::new(),
            selected: None,
        }
    }
    pub fn add(&mut self, item: &str) {
        unsafe {
            SendMessageW(self.hwnd, 0x0143, 0, to_wide(item).as_ptr() as isize);
        }
        self.items.push(item.into());
        if self.selected.is_none() {
            self.selected = Some(0);
        }
    }
    pub fn get_selection(&self) -> Option<i32> {
        unsafe {
            let s = SendMessageW(self.hwnd, 0x0147, 0, 0);
            if s == -1 {
                None
            } else {
                Some(s as i32)
            }
        }
    }
    pub fn set_selection(&mut self, i: i32) {
        unsafe {
            SendMessageW(self.hwnd, 0x014E, i as usize, 0);
        }
        self.selected = Some(i);
    }
    pub fn get_string(&self, i: i32) -> Option<String> {
        self.items.get(i as usize).cloned()
    }
    pub fn clear(&mut self) {
        unsafe {
            SendMessageW(self.hwnd, 0x014B, 0, 0);
        }
        self.items.clear();
        self.selected = None;
    }
    pub fn get_count(&self) -> usize {
        self.items.len()
    }
    pub fn id(&self) -> u16 {
        self.id
    }

    /// Insert an item at the given index (shifting existing items right).
    pub fn insert(&mut self, index: usize, item: &str) {
        unsafe {
            SendMessageW(
                self.hwnd,
                0x014A, /* CB_INSERTSTRING */
                index,
                to_wide(item).as_ptr() as isize,
            );
        }
        self.items.insert(index.min(self.items.len()), item.into());
    }

    /// Remove an item at the given index.
    pub fn remove(&mut self, index: i32) {
        if index >= 0 && (index as usize) < self.items.len() {
            unsafe {
                SendMessageW(
                    self.hwnd,
                    0x0144, /* CB_DELETESTRING */
                    index as usize,
                    0,
                );
            }
            self.items.remove(index as usize);
        }
    }

    pub fn on_selection_change<F: Fn(i32) + 'static>(&self, parent: &Frame, callback: F) {
        events::register_hwnd(parent.hwnd(), self.hwnd, move |ev| {
            if let events::WidgetEvent::SelectionChanged(idx) = ev {
                (callback)(idx);
            }
        });
    }
    pub fn set_size(&self, w: i32, h: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                0,
                0,
                w,
                h,
                SWP_NOZORDER | SWP_NOMOVE,
            );
        }
    }
    pub fn set_position(&self, x: i32, y: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOZORDER | SWP_NOSIZE,
            );
        }
    }
    pub fn as_widget_ref(&self) -> crate::core::widget::WidgetRef {
        super::window_trait::make_widget_ref(self.hwnd)
    }
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}
impl Drop for Choice {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

// ============================================================================
// ListBox
// ============================================================================

pub struct ListBox {
    hwnd: HWND,
    id: u16,
    items: Vec<String>,
}

impl ListBox {
    pub fn new<P: WidgetParent>(parent: &P) -> Self {
        let id = next_id();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide("LISTBOX").as_ptr(),
                to_wide("").as_ptr(),
                0x00A0_0003,
                0,
                0,
                150,
                200,
                parent.hwnd(),
                id as isize as *mut std::ffi::c_void,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            )
        };
        Self {
            hwnd,
            id,
            items: Vec::new(),
        }
    }
    pub fn add(&mut self, item: &str) {
        unsafe {
            SendMessageW(self.hwnd, 0x0180, 0, to_wide(item).as_ptr() as isize);
        }
        self.items.push(item.into());
    }
    pub fn get_selection(&self) -> Option<i32> {
        unsafe {
            let s = SendMessageW(self.hwnd, 0x0188, 0, 0);
            if s == -1 {
                None
            } else {
                Some(s as i32)
            }
        }
    }
    pub fn set_selection(&self, i: i32) {
        unsafe {
            SendMessageW(self.hwnd, 0x0186, i as usize, 0);
        }
    }
    pub fn get_string(&self, i: i32) -> Option<String> {
        self.items.get(i as usize).cloned()
    }
    pub fn clear(&mut self) {
        unsafe {
            SendMessageW(self.hwnd, 0x0184, 0, 0);
        }
        self.items.clear();
    }
    pub fn get_count(&self) -> usize {
        self.items.len()
    }
    pub fn delete(&mut self, i: i32) {
        unsafe {
            SendMessageW(self.hwnd, 0x0182, i as usize, 0);
        }
        if (i as usize) < self.items.len() {
            self.items.remove(i as usize);
        }
    }
    pub fn id(&self) -> u16 {
        self.id
    }

    /// Insert an item at the given index.
    pub fn insert(&mut self, index: usize, item: &str) {
        unsafe {
            SendMessageW(
                self.hwnd,
                0x0181, /* LB_INSERTSTRING */
                index,
                to_wide(item).as_ptr() as isize,
            );
        }
        self.items.insert(index.min(self.items.len()), item.into());
    }

    /// Create a multi-select ListBox (LBS_EXTENDEDSEL style).
    pub fn multi_select<P: WidgetParent>(parent: &P) -> Self {
        let id = next_id();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide("LISTBOX").as_ptr(),
                to_wide("").as_ptr(),
                0x00A0_0003 | 0x0000_0800, /* LBS_EXTENDEDSEL */
                0,
                0,
                150,
                200,
                parent.hwnd(),
                id as isize as *mut std::ffi::c_void,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            )
        };
        Self {
            hwnd,
            id,
            items: Vec::new(),
        }
    }

    /// Register a handler that fires on double-click.
    pub fn on_double_click<F: Fn() + 'static>(&self, parent: &Frame, callback: F) {
        // Double-click is a BN_DBLCLK notification (code 5) sent via WM_COMMAND.
        // We use register_hwnd which routes through the WM_COMMAND code.
        let id = self.id;
        events::register_command(parent.hwnd(), id, move |_| {
            (callback)();
        });
    }

    /// Get all selected indices (for multi-select mode).
    pub fn get_selections(&self) -> Vec<usize> {
        // LB_GETSELCOUNT = 0x0190
        let count = unsafe { SendMessageW(self.hwnd, 0x0190, 0, 0) } as usize;
        if count == 0 {
            return Vec::new();
        }
        let mut buf = vec![0i32; count];
        // LB_GETSELITEMS = 0x0191
        let filled =
            unsafe { SendMessageW(self.hwnd, 0x0191, count, buf.as_mut_ptr() as isize) } as usize;
        buf[..filled].iter().map(|&i| i as usize).collect()
    }

    /// Set the font of the list box via `WM_SETFONT`.
    /// The `font` is a raw HFONT handle obtained from `CreateFontIndirectW`.
    pub fn set_font(&self, font: isize) {
        unsafe {
            SendMessageW(self.hwnd, 0x0030 /* WM_SETFONT */, font as usize, 1);
        }
    }

    /// Set the appearance of the list box (add or remove borders).
    /// `bordered = true` adds `WS_BORDER`; `bordered = false` removes it.
    pub fn set_appearance(&self, bordered: bool) {
        unsafe {
            let style = GetWindowLongW(self.hwnd, -16 /* GWL_STYLE */) as u32;
            if bordered {
                SetWindowLongW(self.hwnd, -16, (style | 0x00800000/* WS_BORDER */) as i32);
            } else {
                SetWindowLongW(self.hwnd, -16, (style & !0x00800000) as i32);
            }
        }
    }

    pub fn on_selection_change<F: Fn(i32) + 'static>(&self, parent: &Frame, callback: F) {
        events::register_hwnd(parent.hwnd(), self.hwnd, move |ev| {
            if let events::WidgetEvent::SelectionChanged(idx) = ev {
                (callback)(idx);
            }
        });
    }
    pub fn set_size(&self, w: i32, h: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                0,
                0,
                w,
                h,
                SWP_NOZORDER | SWP_NOMOVE,
            );
        }
    }
    pub fn set_position(&self, x: i32, y: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOZORDER | SWP_NOSIZE,
            );
        }
    }
    pub fn as_widget_ref(&self) -> crate::core::widget::WidgetRef {
        super::window_trait::make_widget_ref(self.hwnd)
    }
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}
impl Drop for ListBox {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

// ============================================================================
// ListView (report mode)
// ============================================================================

pub struct ListView {
    hwnd: HWND,
    id: u16,
    columns: Vec<String>,
    items: Vec<Vec<String>>,
}

impl ListView {
    pub fn new<P: WidgetParent>(parent: &P) -> Self {
        let id = next_id();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide("SysListView32").as_ptr(),
                to_wide("").as_ptr(),
                WS_CHILD | WS_VISIBLE | 0x0001 | 0x0004 | 0x0020,
                0,
                0,
                200,
                150,
                parent.hwnd(),
                id as isize as *mut std::ffi::c_void,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            )
        };
        Self {
            hwnd,
            id,
            columns: Vec::new(),
            items: Vec::new(),
        }
    }
    pub fn add_column(&mut self, title: &str, width: i32) {
        let mut wide = to_wide(title);
        unsafe {
            let mut lvc = std::mem::zeroed::<LV_COLUMNW>();
            lvc.mask = 0x0001 | 0x0002 | 0x0004;
            lvc.cx = width;
            lvc.pszText = wide.as_mut_ptr();
            SendMessageW(
                self.hwnd,
                0x101B,
                self.columns.len(),
                &mut lvc as *mut _ as isize,
            );
        }
        self.columns.push(title.into());
    }
    pub fn add_item(&mut self, text: &str) {
        let mut wide = to_wide(text);
        unsafe {
            let mut lvi = std::mem::zeroed::<LV_ITEMW>();
            lvi.mask = 0x0001;
            lvi.iItem = self.items.len() as i32;
            lvi.pszText = wide.as_mut_ptr();
            SendMessageW(self.hwnd, 0x104D, 0, &mut lvi as *mut _ as isize);
        }
        self.items.push(vec![text.into()]);
    }
    pub fn add_subitem(&mut self, text: &str, column: usize) {
        if let Some(last) = self.items.last_mut() {
            if column >= last.len() {
                last.push(text.into());
            } else {
                last[column] = text.into();
            }
            let mut wide = to_wide(text);
            unsafe {
                let mut lvi = std::mem::zeroed::<LV_ITEMW>();
                lvi.mask = 0x0001;
                lvi.iItem = (self.items.len() - 1) as i32;
                lvi.iSubItem = column as i32;
                lvi.pszText = wide.as_mut_ptr();
                SendMessageW(self.hwnd, 0x1046, 0, &mut lvi as *mut _ as isize);
            }
        }
    }
    pub fn get_selection(&self) -> Option<i32> {
        unsafe {
            let s = SendMessageW(self.hwnd, 0x103F, 0, 0) as i32;
            if s == -1 {
                None
            } else {
                Some(s)
            }
        }
    }
    pub fn get_count(&self) -> usize {
        self.items.len()
    }
    pub fn id(&self) -> u16 {
        self.id
    }

    /// Sort items by the given column (ascending string comparison).
    /// This is a simple in-memory sort — it rebuilds the list view
    /// from the internal `items` vec.
    pub fn sort_by_column(&mut self, col: usize) {
        self.items.sort_by(|a, b| {
            let a_str = a.get(col).map(|s| s.as_str()).unwrap_or("");
            let b_str = b.get(col).map(|s| s.as_str()).unwrap_or("");
            a_str.cmp(b_str)
        });
        // Re-populate the ListView from the sorted data.
        unsafe {
            SendMessageW(self.hwnd, 0x1009 /* LVM_DELETEALLITEMS */, 0, 0);
        }
        for (row_i, row) in self.items.iter().enumerate() {
            let mut wide = to_wide(&row[0]);
            unsafe {
                let mut lvi = std::mem::zeroed::<LV_ITEMW>();
                lvi.mask = 0x0001;
                lvi.iItem = row_i as i32;
                lvi.pszText = wide.as_mut_ptr();
                SendMessageW(
                    self.hwnd,
                    0x104D, /* LVM_INSERTITEM */
                    0,
                    &mut lvi as *mut _ as isize,
                );
            }
            for (col_i, cell) in row.iter().enumerate().skip(1) {
                let mut wide = to_wide(cell);
                unsafe {
                    let mut lvi = std::mem::zeroed::<LV_ITEMW>();
                    lvi.mask = 0x0001;
                    lvi.iItem = row_i as i32;
                    lvi.iSubItem = col_i as i32;
                    lvi.pszText = wide.as_mut_ptr();
                    SendMessageW(
                        self.hwnd,
                        0x1046, /* LVM_SETITEMTEXT */
                        0,
                        &mut lvi as *mut _ as isize,
                    );
                }
            }
        }
    }

    pub fn set_size(&self, w: i32, h: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                0,
                0,
                w,
                h,
                SWP_NOZORDER | SWP_NOMOVE,
            );
        }
    }
    pub fn set_position(&self, x: i32, y: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOZORDER | SWP_NOSIZE,
            );
        }
    }
    pub fn as_widget_ref(&self) -> crate::core::widget::WidgetRef {
        super::window_trait::make_widget_ref(self.hwnd)
    }
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}

#[repr(C)]
#[allow(non_snake_case)]
struct LV_COLUMNW {
    mask: u32,
    fmt: i32,
    cx: i32,
    pszText: *mut u16,
    cchTextMax: i32,
    iSubItem: i32,
    iImage: i32,
    iOrder: i32,
}

#[repr(C)]
#[allow(non_snake_case)]
struct LV_ITEMW {
    mask: u32,
    iItem: i32,
    iSubItem: i32,
    state: u32,
    stateMask: u32,
    pszText: *mut u16,
    cchTextMax: i32,
    iImage: i32,
    lParam: isize,
}

impl Drop for ListView {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

// ============================================================================
// ListBoxUWP (stub, uses LISTBOX with LBS_USETABSTOPS)
// ============================================================================

pub struct ListBoxUWP {
    hwnd: HWND,
    id: u16,
    items: Vec<String>,
}

impl ListBoxUWP {
    pub fn new<P: WidgetParent>(parent: &P) -> Self {
        let id = next_id();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide("LISTBOX").as_ptr(),
                to_wide("").as_ptr(),
                0x00A0_0003 | 0x0080,
                0,
                0,
                150,
                200,
                parent.hwnd(),
                id as isize as *mut std::ffi::c_void,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            )
        };
        Self {
            hwnd,
            id,
            items: Vec::new(),
        }
    }
    pub fn add(&mut self, item: &str) {
        unsafe {
            SendMessageW(self.hwnd, 0x0180, 0, to_wide(item).as_ptr() as isize);
        }
        self.items.push(item.into());
    }
    pub fn get_selection(&self) -> Option<i32> {
        unsafe {
            let s = SendMessageW(self.hwnd, 0x0188, 0, 0);
            if s == -1 {
                None
            } else {
                Some(s as i32)
            }
        }
    }
    pub fn get_string(&self, i: i32) -> Option<String> {
        self.items.get(i as usize).cloned()
    }
    pub fn get_count(&self) -> usize {
        self.items.len()
    }
    pub fn set_size(&self, w: i32, h: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                0,
                0,
                w,
                h,
                SWP_NOZORDER | SWP_NOMOVE,
            );
        }
    }
    pub fn set_position(&self, x: i32, y: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOZORDER | SWP_NOSIZE,
            );
        }
    }
    pub fn as_widget_ref(&self) -> crate::core::widget::WidgetRef {
        super::window_trait::make_widget_ref(self.hwnd)
    }
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}
impl Drop for ListBoxUWP {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

// ============================================================================
// Slider (Trackbar)
// ============================================================================

pub struct Slider {
    hwnd: HWND,
    id: u16,
    min: i32,
    max: i32,
}

impl Slider {
    pub fn new<P: WidgetParent>(parent: &P, min: i32, max: i32) -> Self {
        let id = next_id();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide("msctls_trackbar32").as_ptr(),
                to_wide("").as_ptr(),
                WS_CHILD_VISIBLE | 0x0001,
                0,
                0,
                200,
                30,
                parent.hwnd(),
                id as isize as *mut std::ffi::c_void,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            )
        };
        let slider = Self { hwnd, id, min, max };
        slider.set_range(min, max);
        slider
    }
    pub fn set_value(&self, value: i32) {
        let v = value.clamp(self.min, self.max);
        unsafe {
            SendMessageW(self.hwnd, 0x0405, 1, v as isize);
        }
    }
    pub fn get_value(&self) -> i32 {
        unsafe { SendMessageW(self.hwnd, 0x0400, 0, 0) as i32 }
    }
    pub fn set_range(&self, min: i32, max: i32) {
        unsafe {
            SendMessageW(self.hwnd, 0x0407, 0, min as isize);
            SendMessageW(self.hwnd, 0x0408, 0, max as isize);
        }
    }
    /// Create a vertical slider.
    pub fn new_vertical<P: WidgetParent>(parent: &P, min: i32, max: i32) -> Self {
        let id = next_id();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide("msctls_trackbar32").as_ptr(),
                to_wide("").as_ptr(),
                WS_CHILD_VISIBLE | 0x0001 | 0x0002, /* TBS_VERT */
                0,
                0,
                30,
                200,
                parent.hwnd(),
                id as isize as *mut std::ffi::c_void,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            )
        };
        let slider = Self { hwnd, id, min, max };
        slider.set_range(min, max);
        slider
    }

    /// Get the current minimum value.
    pub fn get_min(&self) -> i32 {
        self.min
    }

    /// Get the current maximum value.
    pub fn get_max(&self) -> i32 {
        self.max
    }

    /// Get the current `(min, max)` range.
    pub fn get_range(&self) -> (i32, i32) {
        (self.min, self.max)
    }

    /// Set the tick frequency (how often a tick mark appears).
    pub fn set_tick_freq(&self, freq: i32) {
        unsafe {
            SendMessageW(
                self.hwnd,
                0x0410, /* TBM_SETTICFREQ */
                freq as usize,
                0,
            );
        }
    }

    /// Set the page size (how far the thumb moves on Page Up/Down).
    pub fn set_page_size(&self, page: i32) {
        unsafe {
            SendMessageW(
                self.hwnd,
                0x0412, /* TBM_SETPAGESIZE */
                0,
                page as isize,
            );
        }
    }

    /// Set the line size (how far the thumb moves on arrow keys).
    pub fn set_line_size(&self, line: i32) {
        unsafe {
            SendMessageW(
                self.hwnd,
                0x0413, /* TBM_SETLINESIZE */
                0,
                line as isize,
            );
        }
    }

    pub fn id(&self) -> u16 {
        self.id
    }

    pub fn on_change<F: Fn(i32) + 'static>(&self, parent: &Frame, callback: F) {
        events::register_hwnd(parent.hwnd(), self.hwnd, move |ev| {
            if let events::WidgetEvent::ValueChanged(v) = ev {
                (callback)(v);
            }
        });
    }
    pub fn set_size(&self, w: i32, h: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                0,
                0,
                w,
                h,
                SWP_NOZORDER | SWP_NOMOVE,
            );
        }
    }
    pub fn set_position(&self, x: i32, y: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOZORDER | SWP_NOSIZE,
            );
        }
    }
    pub fn as_widget_ref(&self) -> crate::core::widget::WidgetRef {
        super::window_trait::make_widget_ref(self.hwnd)
    }
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}
impl Drop for Slider {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

// ============================================================================
// Gauge (Progress Bar)
// ============================================================================

pub struct Gauge {
    hwnd: HWND,
    id: u16,
    min: i32,
    max: i32,
}

impl Gauge {
    pub fn new<P: WidgetParent>(parent: &P) -> Self {
        let id = next_id();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide("msctls_progress32").as_ptr(),
                to_wide("").as_ptr(),
                WS_CHILD_VISIBLE | 0x01,
                0,
                0,
                200,
                20,
                parent.hwnd(),
                id as isize as *mut std::ffi::c_void,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            )
        };
        Self {
            hwnd,
            id,
            min: 0,
            max: 100,
        }
    }
    pub fn set_value(&self, value: i32) {
        let v = value.clamp(self.min, self.max);
        unsafe {
            SendMessageW(self.hwnd, 0x0402, v as usize, 0);
        }
    }
    pub fn get_value(&self) -> i32 {
        unsafe { SendMessageW(self.hwnd, 0x0408, 0, 0) as i32 }
    }
    pub fn set_range(&self, min: i32, max: i32) {
        unsafe {
            SendMessageW(self.hwnd, 0x0406, min as usize, max as isize);
        }
    }
    pub fn id(&self) -> u16 {
        self.id
    }

    /// Start or stop marquee (indeterminate) mode.
    pub fn set_marquee(&self, on: bool, period_ms: u32) {
        unsafe {
            SendMessageW(
                self.hwnd,
                0x040A, /* PBM_SETMARQUEE */
                on as usize,
                period_ms as isize,
            );
        }
    }

    pub fn set_size(&self, w: i32, h: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                0,
                0,
                w,
                h,
                SWP_NOZORDER | SWP_NOMOVE,
            );
        }
    }
    pub fn set_position(&self, x: i32, y: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOZORDER | SWP_NOSIZE,
            );
        }
    }
    pub fn as_widget_ref(&self) -> crate::core::widget::WidgetRef {
        super::window_trait::make_widget_ref(self.hwnd)
    }
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}
impl Drop for Gauge {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

// ============================================================================
// RadioBox
// ============================================================================

pub struct RadioBox {
    hwnd: HWND, // the group box container
    id: u16,
    labels: Vec<String>,
    radio_ids: Vec<u16>, // control id of each radio button
    selected: i32,
}

impl RadioBox {
    pub fn new<P: WidgetParent>(parent: &P, label: &str, choices: &[&str]) -> Self {
        let id = next_id();
        let parent_hwnd = parent.hwnd();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide("BUTTON").as_ptr(),
                to_wide(label).as_ptr(),
                WS_CHILD_VISIBLE | 0x0000_0007,
                0,
                0,
                200,
                150,
                parent_hwnd,
                id as isize as *mut std::ffi::c_void,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            )
        };

        // Create individual radio buttons inside the group box.
        let mut radio_ids = Vec::new();
        for (i, choice) in choices.iter().enumerate() {
            let rid = next_id();
            let mut style = WS_CHILD_VISIBLE | 0x0000_0004 /* BS_RADIOBUTTON */;
            if i == 0 {
                style |= 0x00020000; // WS_GROUP — first button starts a group
            }
            unsafe {
                CreateWindowExW(
                    0,
                    to_wide("BUTTON").as_ptr(),
                    to_wide(choice).as_ptr(),
                    style,
                    12,
                    18 + (i as i32 * 24),
                    176,
                    20,
                    hwnd,
                    rid as isize as *mut std::ffi::c_void,
                    GetModuleHandleW(ptr::null()),
                    ptr::null_mut(),
                );
            }
            radio_ids.push(rid);
        }

        Self {
            hwnd,
            id,
            labels: choices.iter().map(|s| (*s).into()).collect(),
            radio_ids,
            selected: -1,
        }
    }

    pub fn set_selection(&mut self, i: i32) {
        if i >= 0 && (i as usize) < self.radio_ids.len() {
            let rid = self.radio_ids[i as usize];
            let parent_hwnd = self.hwnd;
            unsafe {
                // Uncheck all radio buttons first.
                for &r in &self.radio_ids {
                    let btn_hwnd = GetDlgItem(parent_hwnd, r as i32);
                    if !btn_hwnd.is_null() {
                        SendMessageW(btn_hwnd, 0x00F1 /* BM_SETCHECK */, 0, 0);
                    }
                }
                // Check the selected one.
                let btn_hwnd = GetDlgItem(parent_hwnd, rid as i32);
                if !btn_hwnd.is_null() {
                    SendMessageW(btn_hwnd, 0x00F1, 1, 0);
                }
            }
            self.selected = i;
        }
    }

    pub fn get_selection(&self) -> i32 {
        self.selected
    }
    pub fn get_string(&self, i: i32) -> Option<String> {
        self.labels.get(i as usize).cloned()
    }
    pub fn get_count(&self) -> usize {
        self.labels.len()
    }
    pub fn id(&self) -> u16 {
        self.id
    }

    /// Register a handler invoked when the selection changes.
    pub fn on_selection_change<F: Fn(i32) + 'static>(&self, parent: &Frame, callback: F) {
        let id = self.id;
        events::register_command(parent.hwnd(), id, move |ev| {
            if let events::WidgetEvent::SelectionChanged(idx) = ev {
                (callback)(idx);
            }
        });
    }

    /// Enable or disable a specific radio button.
    pub fn enable_choice(&self, i: usize, enabled: bool) {
        if i < self.radio_ids.len() {
            let rid = self.radio_ids[i];
            unsafe {
                let btn_hwnd = GetDlgItem(self.hwnd, rid as i32);
                if !btn_hwnd.is_null() {
                    // EnableWindow is in Win32::UI::Input::KeyboardAndMouse
                    // but the function is exported from user32.dll; we call
                    // it via SendMessageW(BM_SETSTATE) as a workaround.
                    SendMessageW(btn_hwnd, 0x00F3 /* BM_SETSTATE */, enabled as usize, 0);
                }
            }
        }
    }

    pub fn set_size(&self, w: i32, h: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                0,
                0,
                w,
                h,
                SWP_NOZORDER | SWP_NOMOVE,
            );
        }
    }
    pub fn set_position(&self, x: i32, y: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOZORDER | SWP_NOSIZE,
            );
        }
    }
    pub fn as_widget_ref(&self) -> crate::core::widget::WidgetRef {
        super::window_trait::make_widget_ref(self.hwnd)
    }
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}

impl Drop for RadioBox {
    fn drop(&mut self) {
        unsafe {
            // Destroy child radio buttons first.
            for &rid in &self.radio_ids {
                let btn_hwnd = GetDlgItem(self.hwnd, rid as i32);
                if !btn_hwnd.is_null() {
                    DestroyWindow(btn_hwnd);
                }
            }
            DestroyWindow(self.hwnd);
        }
    }
}

// ============================================================================
// ComboBox (editable)
// ============================================================================

pub struct ComboBox {
    hwnd: HWND,
    id: u16,
    items: Vec<String>,
    selected: Option<i32>,
}

impl ComboBox {
    pub fn new<P: WidgetParent>(parent: &P) -> Self {
        let id = next_id();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide("COMBOBOX").as_ptr(),
                to_wide("").as_ptr(),
                WS_CHILD_VISIBLE | 0x0002 | 0x0080,
                0,
                0,
                150,
                200,
                parent.hwnd(),
                id as isize as *mut std::ffi::c_void,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            )
        };
        Self {
            hwnd,
            id,
            items: Vec::new(),
            selected: None,
        }
    }
    pub fn add(&mut self, item: &str) {
        unsafe {
            SendMessageW(self.hwnd, 0x0143, 0, to_wide(item).as_ptr() as isize);
        }
        self.items.push(item.into());
    }
    pub fn get_selection(&self) -> Option<i32> {
        unsafe {
            let s = SendMessageW(self.hwnd, 0x0147, 0, 0);
            if s == -1 {
                None
            } else {
                Some(s as i32)
            }
        }
    }
    pub fn set_selection(&mut self, i: i32) {
        unsafe {
            SendMessageW(self.hwnd, 0x014E, i as usize, 0);
        }
        self.selected = Some(i);
    }
    pub fn set_value(&self, t: &str) {
        unsafe {
            SetWindowTextW(self.hwnd, to_wide(t).as_ptr());
        }
    }
    pub fn get_value(&self) -> String {
        unsafe {
            let len = GetWindowTextLengthW(self.hwnd);
            if len <= 0 {
                return String::new();
            }
            let mut buf: Vec<u16> = vec![0; len as usize + 1];
            GetWindowTextW(self.hwnd, buf.as_mut_ptr(), len + 1);
            String::from_utf16_lossy(&buf[..len as usize])
        }
    }
    pub fn clear(&mut self) {
        unsafe {
            SendMessageW(self.hwnd, 0x014B, 0, 0);
        }
        self.items.clear();
        self.selected = None;
    }
    pub fn get_count(&self) -> usize {
        self.items.len()
    }
    pub fn id(&self) -> u16 {
        self.id
    }

    /// Insert an item at the given index.
    pub fn insert(&mut self, index: usize, item: &str) {
        unsafe {
            SendMessageW(
                self.hwnd,
                0x014A, /* CB_INSERTSTRING */
                index,
                to_wide(item).as_ptr() as isize,
            );
        }
        self.items.insert(index.min(self.items.len()), item.into());
    }

    /// Remove the item at the given index.
    pub fn remove(&mut self, index: i32) {
        if index >= 0 && (index as usize) < self.items.len() {
            unsafe {
                SendMessageW(
                    self.hwnd,
                    0x0144, /* CB_DELETESTRING */
                    index as usize,
                    0,
                );
            }
            self.items.remove(index as usize);
        }
    }

    pub fn on_selection_change<F: Fn(i32) + 'static>(&self, parent: &Frame, callback: F) {
        events::register_hwnd(parent.hwnd(), self.hwnd, move |ev| {
            if let events::WidgetEvent::SelectionChanged(idx) = ev {
                (callback)(idx);
            }
        });
    }
    pub fn on_text_change<F: Fn(String) + 'static>(&self, parent: &Frame, callback: F) {
        events::register_hwnd(parent.hwnd(), self.hwnd, move |ev| {
            if let events::WidgetEvent::TextChanged(text) = ev {
                (callback)(text);
            }
        });
    }
    pub fn set_size(&self, w: i32, h: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                0,
                0,
                w,
                h,
                SWP_NOZORDER | SWP_NOMOVE,
            );
        }
    }
    pub fn set_position(&self, x: i32, y: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOZORDER | SWP_NOSIZE,
            );
        }
    }
    pub fn as_widget_ref(&self) -> crate::core::widget::WidgetRef {
        super::window_trait::make_widget_ref(self.hwnd)
    }
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}
impl Drop for ComboBox {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

// ============================================================================
// StatusBar
// ============================================================================

pub struct StatusBar {
    hwnd: HWND,
    id: u16,
    parts: Vec<String>,
}

impl StatusBar {
    pub fn new<P: WidgetParent>(parent: &P) -> Self {
        let id = next_id();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide("msctls_statusbar32").as_ptr(),
                to_wide("").as_ptr(),
                WS_CHILD_VISIBLE,
                0,
                0,
                300,
                20,
                parent.hwnd(),
                id as isize as *mut std::ffi::c_void,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            )
        };
        Self {
            hwnd,
            id,
            parts: vec!["Ready".into()],
        }
    }
    pub fn set_part_count(&mut self, n: usize) {
        self.parts.resize(n, String::new());
        let n = n as i32;
        let mut edges: Vec<i32> = (1..=n).map(|i| i * 200).collect();
        edges.push(-1);
        unsafe {
            SendMessageW(
                self.hwnd,
                0x0404,
                (n + 1) as usize,
                edges.as_mut_ptr() as isize,
            );
        }
    }
    pub fn set_part_text(&mut self, part: usize, text: &str) {
        if part >= self.parts.len() {
            self.set_part_count(part + 1);
        }
        let wide = to_wide(text);
        unsafe {
            SendMessageW(self.hwnd, 0x040B, part, wide.as_ptr() as isize);
        }
        self.parts[part] = text.into();
    }
    pub fn get_part_text(&self, part: usize) -> String {
        self.parts.get(part).cloned().unwrap_or_default()
    }
    pub fn set_status_text(&mut self, text: &str) {
        self.set_part_text(0, text);
    }
    pub fn get_status_text(&self) -> String {
        self.get_part_text(0)
    }
    pub fn id(&self) -> u16 {
        self.id
    }

    /// Get the number of parts in the status bar.
    pub fn get_fields_count(&self) -> usize {
        self.parts.len()
    }
    pub fn as_widget_ref(&self) -> crate::core::widget::WidgetRef {
        super::window_trait::make_widget_ref(self.hwnd)
    }
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}
impl Drop for StatusBar {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

// ============================================================================
// Tab (SysTabControl32)
// ============================================================================

pub struct TabPage {
    pub label: String,
    pub hwnd: HWND,
    pub panel_hwnd: Option<HWND>,
}
pub struct Tab {
    hwnd: HWND,
    pages: Vec<TabPage>,
}

impl Tab {
    pub fn new<P: WidgetParent>(parent: &P) -> Self {
        let id = next_id();
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                to_wide("SysTabControl32").as_ptr(),
                to_wide("").as_ptr(),
                WS_CHILD | WS_VISIBLE | 0x0200, /* TCS_MULTILINE (TCS_TABS is 0) */
                0,
                0,
                200,
                100,
                parent.hwnd(),
                id as isize as *mut std::ffi::c_void,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            )
        };
        Self {
            hwnd,
            pages: Vec::new(),
        }
    }

    pub fn add_page(&mut self, label: &str, _panel: &Panel) {
        let mut wide = to_wide(label);
        unsafe {
            let mut item = std::mem::zeroed::<TCITEM>();
            item.mask = 0x0001; // TCIF_TEXT
            item.pszText = wide.as_mut_ptr();
            item.cchTextMax = label.len() as i32;
            let index = self.pages.len() as i32;
            SendMessageW(
                self.hwnd,
                0x130A,
                index as usize,
                &mut item as *mut _ as isize,
            );
            let panel_hwnd = CreateWindowExW(
                0,
                to_wide("static").as_ptr(),
                to_wide("").as_ptr(),
                WS_CHILD | WS_VISIBLE,
                0,
                0,
                200,
                100,
                self.hwnd,
                ptr::null_mut(),
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            );
            if index == 0 {
                ShowWindow(panel_hwnd, SW_SHOW);
            } else {
                ShowWindow(panel_hwnd, SW_HIDE);
            }
            self.pages.push(TabPage {
                label: label.into(),
                hwnd: self.hwnd,
                panel_hwnd: Some(panel_hwnd),
            });
        }
    }

    pub fn set_selection(&mut self, index: usize) {
        if index < self.pages.len() {
            unsafe {
                for (i, page) in self.pages.iter().enumerate() {
                    if let Some(ph) = page.panel_hwnd {
                        if i == index {
                            ShowWindow(ph, SW_SHOW);
                        } else {
                            ShowWindow(ph, SW_HIDE);
                        }
                    }
                }
                SendMessageW(self.hwnd, 0x130C, index, 0);
            }
        }
    }
    pub fn get_selection(&self) -> usize {
        unsafe { SendMessageW(self.hwnd, 0x130B, 0, 0) as usize }
    }
    pub fn set_size(&self, x: i32, y: i32, w: i32, h: i32) {
        unsafe {
            SetWindowPos(self.hwnd, ptr::null_mut(), x, y, w, h, 0x0004);
            for page in &self.pages {
                if let Some(ph) = page.panel_hwnd {
                    SetWindowPos(ph, ptr::null_mut(), 0, 30, w, h - 30, 0x0004);
                }
            }
        }
    }
    pub fn set_position(&self, x: i32, y: i32) {
        unsafe {
            SetWindowPos(self.hwnd, ptr::null_mut(), x, y, 0, 0, 0x0004 | 0x0002);
        }
    }
    pub fn get_page_hwnd(&self, i: usize) -> Option<HWND> {
        self.pages.get(i).and_then(|p| p.panel_hwnd)
    }
    pub fn on_page_changed<F: Fn(usize) + 'static>(&self, parent: &Frame, callback: F) {
        events::register_hwnd(parent.hwnd(), self.hwnd, move |ev| {
            if let events::WidgetEvent::PageChanged(idx) = ev {
                (callback)(idx);
            }
        });
    }
    pub fn as_widget_ref(&self) -> crate::core::widget::WidgetRef {
        super::window_trait::make_widget_ref(self.hwnd)
    }
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }
}

#[repr(C)]
#[allow(non_snake_case)]
#[allow(clippy::upper_case_acronyms)] // mirrors the Win32 SDK struct name
struct TCITEM {
    mask: u32,
    dwState: u32,
    dwStateMask: u32,
    pszText: *mut u16,
    cchTextMax: i32,
    iImage: i32,
    lParam: isize,
}

impl Drop for Tab {
    fn drop(&mut self) {
        unsafe {
            for page in &self.pages {
                if let Some(ph) = page.panel_hwnd {
                    DestroyWindow(ph);
                }
            }
            DestroyWindow(self.hwnd);
        }
    }
}
