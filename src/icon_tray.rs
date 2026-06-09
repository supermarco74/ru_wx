//! System tray (notification area) icon — the ru_wx port of
//! `wxTaskBarIcon`.
//!
//! Backed by Win32 `Shell_NotifyIconW` + a per-instance `WM_APP + n`
//! callback message. The parent `Frame`'s wndproc dispatches the
//! callback messages into the registered tray handler, which then
//! dispatches to the user-supplied closures for left/right click,
//! double-click, and balloon-click events.
//!
//! A context menu can be attached with [`IconTray::set_menu`] — it is
//! shown automatically on right-click (or via
//! `WM_CONTEXTMENU` / `NIN_POPUPOPEN`).

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::frame::Frame;
use crate::menu::Menu;

#[cfg(target_os = "windows")]
use crate::icon::{destroy_hicon, svg_bytes_to_hicon};

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_LARGE_ICON, NIIF_NOSOUND,
    NIM_ADD, NIM_DELETE, NIM_MODIFY, NIM_SETVERSION, NIN_BALLOONUSERCLICK, NIN_POPUPOPEN,
    NIN_SELECT, NOTIFYICONDATAW, NOTIFYICON_VERSION_4,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    HICON, WM_CONTEXTMENU, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN,
};

/// Visual style of a balloon / toast notification.
#[derive(Clone, Copy, Debug)]
pub enum BalloonIcon {
    /// No icon.
    None,
    /// Information (blue).
    Info,
    /// Warning (yellow).
    Warning,
    /// Error (red).
    Error,
    /// User-supplied icon (uses the current tray icon).
    User,
}

impl BalloonIcon {
    #[cfg(target_os = "windows")]
    fn to_flag(self) -> u32 {
        match self {
            BalloonIcon::None => NIIF_NOSOUND,
            BalloonIcon::Info => 1,
            BalloonIcon::Warning => 2,
            BalloonIcon::Error => 3,
            BalloonIcon::User => 4 | NIIF_LARGE_ICON,
        }
    }
}

#[cfg(target_os = "windows")]
fn write_u16_array(dst: &mut [u16], s: &str) {
    dst.iter_mut().for_each(|c| *c = 0);
    for (i, c) in s.encode_utf16().take(dst.len() - 1).enumerate() {
        dst[i] = c;
    }
}

/// Monotonically increasing per-process identifier for `NOTIFYICONDATAW.uID`.
static NEXT_TRAY_UID: AtomicU32 = AtomicU32::new(1);
/// Monotonically increasing per-process `WM_APP + n` callback message id.
static NEXT_TRAY_MSG: AtomicU32 = AtomicU32::new(/* WM_APP */ 0x8000 + 1);

#[cfg(target_os = "windows")]
struct TrayState {
    tooltip: String,
    hicon: HICON,
    on_left_click: Option<Box<dyn FnMut()>>,
    on_left_double_click: Option<Box<dyn FnMut()>>,
    on_right_click: Option<Box<dyn FnMut()>>,
    on_balloon_click: Option<Box<dyn FnMut()>>,
    menu: Option<Menu>,
}

#[cfg(target_os = "windows")]
impl TrayState {
    fn new(hicon: HICON, tooltip: String) -> Self {
        Self {
            tooltip,
            hicon,
            on_left_click: None,
            on_left_double_click: None,
            on_right_click: None,
            on_balloon_click: None,
            menu: None,
        }
    }
}

/// A system tray (notification area) icon.
///
/// Owns the underlying `HICON`, a context menu, and a set of event
/// callbacks (left click / double click / right click / balloon click).
/// On `Drop` the icon is removed from the tray and the `HICON` is
/// destroyed.
pub struct IconTray {
    #[cfg(target_os = "windows")]
    frame: Frame,
    #[cfg(target_os = "windows")]
    uid: u32,
    #[cfg(target_os = "windows")]
    msg: u32,
    #[cfg(target_os = "windows")]
    hicon: HICON,
    #[cfg(target_os = "windows")]
    added: bool,
    #[cfg(target_os = "windows")]
    state: Rc<RefCell<TrayState>>,
}

#[cfg(target_os = "windows")]
impl IconTray {
    /// Create a tray icon and add it to the notification area.
    ///
    /// `svg_bytes` is rendered at `icon_size × icon_size` pixels and used
    /// as the tray icon. Returns `None` if the SVG cannot be rendered.
    pub fn new(frame: &Frame, svg_bytes: &[u8], icon_size: u32) -> Option<Self> {
        let hicon = svg_bytes_to_hicon(svg_bytes, icon_size)?;
        let mut tray = Self::hidden_with_hicon(frame, hicon, String::new());
        tray.added = tray.add_to_tray();
        Some(tray)
    }

    /// Create a tray icon with a 1×1 placeholder icon, ready to be
    /// configured via `set_icon_from_svg_bytes`, `set_tooltip`, and
    /// `set_menu` before being shown with [`IconTray::show`].
    pub fn hidden(frame: &Frame) -> Self {
        // SAFETY: We build a 1x1, 32-bpp DIB-section-free placeholder
        // icon. `CreateBitmap` does **not** require a DC, so we don't
        // pair it with `GetDC`/`ReleaseDC` (the previous implementation
        // did, which was dead code that left a transient screen DC
        // referenced for no reason).
        let hicon = unsafe {
            let hbitmap = windows_sys::Win32::Graphics::Gdi::CreateBitmap(
                1,
                1,
                1,
                32,
                std::ptr::null(),
            );
            let ii = windows_sys::Win32::UI::WindowsAndMessaging::ICONINFO {
                fIcon: 1,
                xHotspot: 0,
                yHotspot: 0,
                hbmMask: std::ptr::null_mut(),
                hbmColor: hbitmap,
            };
            let hicon = windows_sys::Win32::UI::WindowsAndMessaging::CreateIconIndirect(&ii);
            if !hbitmap.is_null() {
                windows_sys::Win32::Graphics::Gdi::DeleteObject(hbitmap);
            }
            hicon
        };
        Self::hidden_with_hicon(frame, hicon, String::new())
    }

    fn hidden_with_hicon(frame: &Frame, hicon: HICON, tooltip: String) -> Self {
        let uid = NEXT_TRAY_UID.fetch_add(1, Ordering::Relaxed);
        let msg = NEXT_TRAY_MSG.fetch_add(1, Ordering::Relaxed);

        let state = Rc::new(RefCell::new(TrayState::new(hicon, tooltip)));

        let tray = Self {
            frame: frame.clone(),
            uid,
            msg,
            hicon,
            added: false,
            state,
        };
        tray.register_callback();
        tray
    }

    fn register_callback(&self) {
        let state = self.state.clone();
        let hwnd = self.frame.hwnd();
        self.frame.register_tray_message_handler(
            self.msg,
            Box::new(move |lparam| {
                let event = lparam & 0xFFFF;
                match event {
                    WM_LBUTTONDOWN | WM_LBUTTONUP | NIN_SELECT => {
                        if let Some(mut h) = state.borrow_mut().on_left_click.take() {
                            h();
                            state.borrow_mut().on_left_click = Some(h);
                        }
                    }
                    WM_LBUTTONDBLCLK => {
                        if let Some(mut h) = state.borrow_mut().on_left_double_click.take() {
                            h();
                            state.borrow_mut().on_left_double_click = Some(h);
                        }
                    }
                    WM_RBUTTONDOWN | WM_CONTEXTMENU | NIN_POPUPOPEN => {
                        // Phase 1: show the context menu (immutable borrow,
                        // dropped at the end of the block).
                        {
                            let s = state.borrow();
                            if let Some(menu) = s.menu.as_ref() {
                                menu.popup_at_cursor(hwnd);
                            }
                        }
                        // Phase 2: fire the user right-click callback.
                        if let Some(mut h) = state.borrow_mut().on_right_click.take() {
                            h();
                            state.borrow_mut().on_right_click = Some(h);
                        }
                    }
                    NIN_BALLOONUSERCLICK => {
                        if let Some(mut h) = state.borrow_mut().on_balloon_click.take() {
                            h();
                            state.borrow_mut().on_balloon_click = Some(h);
                        }
                    }
                    _ => {}
                }
            }),
        );
    }

    /// Replace the current icon. Renders `svg_bytes` at `icon_size` and
    /// issues a `NIM_MODIFY` if the tray is currently shown.
    pub fn set_icon_from_svg_bytes(&mut self, svg_bytes: &[u8], icon_size: u32) -> bool {
        let Some(new_hicon) = svg_bytes_to_hicon(svg_bytes, icon_size) else {
            return false;
        };
        let old = self.hicon;
        self.hicon = new_hicon;
        self.state.borrow_mut().hicon = new_hicon;
        if !old.is_null() {
            destroy_hicon(old);
        }
        if self.added {
            self.modify_tray();
        }
        true
    }

    /// Set the tooltip shown when the user hovers over the icon.
    pub fn set_tooltip(&mut self, tooltip: &str) {
        self.state.borrow_mut().tooltip = tooltip.to_string();
        if self.added {
            self.modify_tray();
        }
    }

    /// Attach a context menu shown on right-click (or `NIN_POPUPOPEN`).
    /// The menu's items must already have their click handlers
    /// registered (e.g. via `Menu::append`).
    pub fn set_menu(&mut self, menu: Menu) {
        self.state.borrow_mut().menu = Some(menu);
    }

    /// Show the tray icon (only relevant after `IconTray::hidden`).
    /// Returns `true` if `Shell_NotifyIconW` succeeded.
    pub fn show(&mut self) -> bool {
        if self.added {
            return true;
        }
        self.added = self.add_to_tray();
        self.added
    }

    /// Hide (remove) the tray icon. You can call `show()` again to
    /// re-add it.
    pub fn hide(&mut self) {
        if !self.added {
            return;
        }
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let nid = self.build_nid(NIF_ICON | NIF_TIP | NIF_MESSAGE);
            Shell_NotifyIconW(NIM_DELETE, &nid);
        }
        self.added = false;
    }

    /// Pop a balloon / toast notification above the tray icon.
    pub fn show_balloon(&self, title: &str, text: &str, icon: BalloonIcon) -> bool {
        if !self.added {
            return false;
        }
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let mut nid = self.build_nid(NIF_ICON | NIF_TIP | NIF_MESSAGE | NIF_INFO);
            write_u16_array(&mut nid.szInfoTitle, title);
            write_u16_array(&mut nid.szInfo, text);
            nid.dwInfoFlags = icon.to_flag();
            Shell_NotifyIconW(NIM_MODIFY, &nid) != 0
        }
    }

    /// Register a callback for left-click (`WM_LBUTTONUP` / `NIN_SELECT`).
    pub fn on_left_click<F: FnMut() + 'static>(&mut self, callback: F) {
        self.state.borrow_mut().on_left_click = Some(Box::new(callback));
    }

    /// Register a callback for left-double-click.
    pub fn on_left_double_click<F: FnMut() + 'static>(&mut self, callback: F) {
        self.state.borrow_mut().on_left_double_click = Some(Box::new(callback));
    }

    /// Register a callback for right-click (just before the context
    /// menu is shown, if any).
    pub fn on_right_click<F: FnMut() + 'static>(&mut self, callback: F) {
        self.state.borrow_mut().on_right_click = Some(Box::new(callback));
    }

    /// Register a callback for the user clicking the balloon / toast.
    pub fn on_balloon_click<F: FnMut() + 'static>(&mut self, callback: F) {
        self.state.borrow_mut().on_balloon_click = Some(Box::new(callback));
    }

    /// The current tray icon id (`NOTIFYICONDATAW.uID`).
    pub fn id(&self) -> u32 {
        self.uid
    }

    // -- private helpers --

    fn add_to_tray(&self) -> bool {
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let mut nid = self.build_nid(NIF_ICON | NIF_TIP | NIF_MESSAGE);
            // Use the modern NOTIFYICON_VERSION_4 protocol so we receive
            // NIN_POPUPOPEN / NIN_SELECT / NIN_BALLOONUSERCLICK etc.
            nid.Anonymous.uVersion = NOTIFYICON_VERSION_4;
            let ok = Shell_NotifyIconW(NIM_ADD, &nid) != 0;
            if ok {
                Shell_NotifyIconW(NIM_SETVERSION, &nid);
            }
            ok
        }
    }

    fn modify_tray(&self) {
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let nid = self.build_nid(NIF_ICON | NIF_TIP | NIF_MESSAGE);
            Shell_NotifyIconW(NIM_MODIFY, &nid);
        }
    }

    fn build_nid(&self, flags: u32) -> NOTIFYICONDATAW {
        let state = self.state.borrow();
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = self.frame.hwnd();
            nid.uID = self.uid;
            nid.uFlags = flags;
            nid.uCallbackMessage = self.msg;
            nid.hIcon = state.hicon;
            write_u16_array(&mut nid.szTip, &state.tooltip);
            nid
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for IconTray {
    fn drop(&mut self) {
        if self.added {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                let nid = self.build_nid(NIF_ICON | NIF_TIP | NIF_MESSAGE);
                Shell_NotifyIconW(NIM_DELETE, &nid);
            }
        }
        if !self.hicon.is_null() {
            destroy_hicon(self.hicon);
        }
        // Detach the message handler from the frame.
        self.frame.unregister_tray_message_handler(self.msg);
    }
}

// ---- Non-Windows stubs ----

#[cfg(not(target_os = "windows"))]
pub struct IconTray;

#[cfg(not(target_os = "windows"))]
impl IconTray {
    pub fn new(_frame: &Frame, _svg_bytes: &[u8], _icon_size: u32) -> Option<Self> {
        None
    }
    pub fn hidden(_frame: &Frame) -> Self {
        Self
    }
    pub fn set_icon_from_svg_bytes(&mut self, _: &[u8], _: u32) -> bool {
        false
    }
    pub fn set_tooltip(&mut self, _: &str) {}
    pub fn set_menu(&mut self, _: Menu) {}
    pub fn show(&mut self) -> bool {
        false
    }
    pub fn hide(&mut self) {}
    pub fn show_balloon(&self, _: &str, _: &str, _: BalloonIcon) -> bool {
        false
    }
    pub fn on_left_click<F: FnMut() + 'static>(&mut self, _: F) {}
    pub fn on_left_double_click<F: FnMut() + 'static>(&mut self, _: F) {}
    pub fn on_right_click<F: FnMut() + 'static>(&mut self, _: F) {}
    pub fn on_balloon_click<F: FnMut() + 'static>(&mut self, _: F) {}
    pub fn id(&self) -> u32 {
        0
    }
}
