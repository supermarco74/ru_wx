//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Modal and modeless dialog windows.
//!
//! A dialog is a secondary top-level window, typically used for user
//! interaction. Modal dialogs block the parent window until dismissed.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::window::frame::Frame;
use crate::core::widget::WidgetRef;

#[cfg(target_os = "windows")]
use crate::platform::win32::{next_control_id, to_wide};
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

/// Dialog style flags
#[cfg(target_os = "windows")]
const DS_MODALFRAME: u32 = 0x80;

/// Shared dialog data accessible from WndProc
pub(crate) struct DialogData {
    #[cfg(target_os = "windows")]
    pub hwnd: HWND,
    pub widgets: Vec<WidgetRef>,
    pub command_handlers: HashMap<u16, Box<dyn FnMut()>>,
    pub result: Option<i32>,
    /// Whether the modal message loop should keep running
    pub modal_running: bool,
}

#[derive(Clone)]
pub struct Dialog {
    pub(crate) inner: Rc<RefCell<DialogData>>,
    #[cfg(target_os = "windows")]
    parent_hwnd: HWND,
}

impl Dialog {
    /// Create a new dialog as a child of the given frame.
    ///
    /// The dialog is a popup window with a title bar, system menu, and modal frame.
    pub fn new(parent: &Frame, title: &str, width: u32, height: u32) -> Self {
        #[cfg(target_os = "windows")]
        {
            let parent_hwnd = parent.hwnd();

            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let hinstance = unsafe { GetModuleHandleW(std::ptr::null()) };
            let class_name = to_wide("RuWxDialogClass");

            // Register dialog window class (idempotent)
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                let wc = WNDCLASSEXW {
                    cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                    style: CS_HREDRAW | CS_VREDRAW,
                    lpfnWndProc: Some(dialog_wnd_proc),
                    cbClsExtra: 0,
                    cbWndExtra: 0,
                    hInstance: hinstance,
                    hIcon: LoadIconW(std::ptr::null_mut(), IDI_APPLICATION),
                    hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
                    hbrBackground: (COLOR_WINDOW + 1) as usize as HBRUSH,
                    lpszMenuName: std::ptr::null(),
                    lpszClassName: class_name.as_ptr(),
                    hIconSm: std::ptr::null_mut(),
                };
                RegisterClassExW(&wc);
            }

            // Create dialog data
            let dialog_data = Box::new(DialogData {
                hwnd: std::ptr::null_mut(),
                widgets: Vec::new(),
                command_handlers: HashMap::new(),
                result: None,
                modal_running: false,
            });
            let dialog_data_ptr = Box::into_raw(dialog_data);

            let title_wide = to_wide(title);
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let hwnd = unsafe {
                CreateWindowExW(
                    0,
                    class_name.as_ptr(),
                    title_wide.as_ptr(),
                    WS_POPUP | WS_CAPTION | WS_SYSMENU | DS_MODALFRAME,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    width as i32,
                    height as i32,
                    parent_hwnd,
                    std::ptr::null_mut(),
                    hinstance,
                    std::ptr::null_mut(),
                )
            };

            // Update dialog data with the hwnd
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                (*dialog_data_ptr).hwnd = hwnd;
            }

            // Wrap in Rc<RefCell>
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let dialog_data = unsafe { Box::from_raw(dialog_data_ptr) };
            let inner = Rc::new(RefCell::new(*dialog_data));

            // Store the Rc pointer in the window's user data for WndProc access
            let inner_clone = inner.clone();
            let raw = Rc::into_raw(inner_clone);
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw as isize);
            }

            let dialog = Dialog { inner, parent_hwnd };
            // Match the parent's Windows 11 look: if the parent has
            // a dark title bar (e.g. built with `with_modern_style`),
            // the dialog inherits the full modern style so the pair
            // looks consistent. Harmless on older Windows releases.
            if crate::window::dwm_style::dark_title_bar_hwnd(parent_hwnd) == Some(true) {
                crate::window::dwm_style::apply_modern_style_hwnd(hwnd);
            }
            dialog
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = (parent, title, width, height);
            Dialog {
                inner: Rc::new(RefCell::new(DialogData {
                    widgets: Vec::new(),
                    command_handlers: HashMap::new(),
                    result: None,
                    modal_running: false,
                })),
            }
        }
    }

    /// Get the native window handle
    #[cfg(target_os = "windows")]
    pub fn hwnd(&self) -> HWND {
        self.inner.borrow().hwnd
    }

    /// Add a widget to the dialog
    pub fn add_widget(&self, widget: WidgetRef) {
        self.inner.borrow_mut().widgets.push(widget);
    }

    /// Register a command handler (for button clicks, etc.)
    pub fn register_command_handler(&self, id: u16, handler: Box<dyn FnMut()>) {
        self.inner.borrow_mut().command_handlers.insert(id, handler);
    }

    /// Show the dialog as modal (blocks parent window until closed).
    ///
    /// Disables the parent window, runs a local message loop, and returns
    /// the result passed to `end_modal()`.
    pub fn show_modal(&self) -> i32 {
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;

            // Disable parent window
            // SAFETY: FFI call to EnableWindow; `hwnd` is a live window owned by this crate.
            unsafe {
                EnableWindow(self.parent_hwnd, 0);
            }

            // Show the dialog
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                ShowWindow(hwnd, SW_SHOW);
                UpdateWindow(hwnd);
            }

            // Mark modal as running
            self.inner.borrow_mut().modal_running = true;

            // Local message loop
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let mut msg: MSG = unsafe { std::mem::zeroed() };
            loop {
                // SAFETY: FFI call to GetMessageW; `hwnd` is a live window and `msg` / `wParam` / `lParam` are valid for that window.
                let ret = unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) };
                if ret <= 0 {
                    // WM_QUIT received — repost and break
                    if ret == 0 {
                        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                        unsafe {
                            PostQuitMessage(msg.wParam as i32);
                        }
                    }
                    break;
                }

                // Check if dialog is still modal
                if !self.inner.borrow().modal_running {
                    break;
                }

                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                unsafe {
                    // IsDialogMessage handles tab navigation and dialog shortcuts
                    if IsDialogMessageW(hwnd, &msg) == 0 {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            }

            // Re-enable parent window
            // SAFETY: FFI call to EnableWindow; `hwnd` is a live window owned by this crate.
            unsafe {
                EnableWindow(self.parent_hwnd, 1);
            }

            // Bring parent to foreground
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                SetForegroundWindow(self.parent_hwnd);
            }

            // Return the result
            self.inner.borrow().result.unwrap_or(0)
        }

        #[cfg(not(target_os = "windows"))]
        {
            self.inner.borrow_mut().modal_running = true;
            0
        }
    }

    /// End the modal dialog with a result code.
    ///
    /// Call this from within a command handler to close the dialog
    /// and return the given value from `show_modal()`.
    pub fn end_modal(&self, result: i32) {
        #[cfg(target_os = "windows")]
        {
            self.inner.borrow_mut().result = Some(result);
            self.inner.borrow_mut().modal_running = false;
            let hwnd = self.inner.borrow().hwnd;
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                DestroyWindow(hwnd);
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            self.inner.borrow_mut().result = Some(result);
            self.inner.borrow_mut().modal_running = false;
        }
    }

    /// Close the dialog (non-modal).
    pub fn close(&self) {
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                DestroyWindow(hwnd);
            }
        }
    }

    /// Convenience: create an OK button inside this dialog and register a handler.
    ///
    /// Returns the button's WidgetRef so it can be added to a sizer.
    pub fn create_button(&self, label: &str) -> (u16, WidgetRef) {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        {
            let hwnd = self.inner.borrow().hwnd;
            let wide_label = to_wide(label);
            let wide_class = to_wide("BUTTON");
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let btn_hwnd = unsafe {
                CreateWindowExW(
                    0,
                    wide_class.as_ptr(),
                    wide_label.as_ptr(),
                    WS_CHILD | WS_VISIBLE | WS_TABSTOP,
                    0,
                    0,
                    80,
                    28,
                    hwnd,
                    id as usize as HMENU,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };

            let widget_ref: WidgetRef = Rc::new(RefCell::new(DialogButtonInner {
                hwnd: btn_hwnd,
                _id: id,
                rect: crate::core::geometry::Rect::new(0, 0, 80, 28),
                visible: true,
                enabled: true,
            }));

            (id, widget_ref)
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = label;
            (
                id,
                Rc::new(RefCell::new(DialogButtonInner {
                    _id: id,
                    rect: crate::core::geometry::Rect::new(0, 0, 80, 28),
                    visible: true,
                    enabled: true,
                })),
            )
        }
    }
}

/// Inner state for a button created inside a Dialog
struct DialogButtonInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    _id: u16,
    rect: crate::core::geometry::Rect,
    visible: bool,
    enabled: bool,
}

impl crate::core::widget::Widget for DialogButtonInner {
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

    fn rect(&self) -> crate::core::geometry::Rect {
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

/// Win32 Window Procedure for Dialog windows
#[cfg(target_os = "windows")]
unsafe extern "system" fn dialog_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as u16;
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                // Bump the strong count before `Rc::from_raw`;
                // see `frame.rs` WM_NOTIFY for the full rationale
                // (the build's `clone + into_raw` leaves the count
                // at 2, so without this bump the second WM_COMMAND
                // would drop the count to 0 and free the backing
                // storage).
                unsafe {
                    Rc::increment_strong_count(ptr as *const RefCell<DialogData>);
                }
                let rc = unsafe { Rc::from_raw(ptr as *const RefCell<DialogData>) };

                // Take handler out temporarily to avoid double borrow
                let handler = rc.borrow_mut().command_handlers.remove(&id);

                if let Some(mut h) = handler {
                    h();
                    rc.borrow_mut().command_handlers.insert(id, h);
                }

                drop(rc);
            }
            0
        }
        WM_CLOSE => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                // Bump the strong count before `Rc::from_raw`;
                // see `frame.rs` WM_NOTIFY for the full rationale.
                unsafe {
                    Rc::increment_strong_count(ptr as *const RefCell<DialogData>);
                }
                let rc = unsafe { Rc::from_raw(ptr as *const RefCell<DialogData>) };
                rc.borrow_mut().modal_running = false;
                drop(rc);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_DESTROY => {
            // Clean up the Rc
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let _ = Rc::from_raw(ptr as *const RefCell<DialogData>);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
