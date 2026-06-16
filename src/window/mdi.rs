//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! MDI (Multiple Document Interface) parent and child frames.
//!
//! MDI is the classic "windows inside a window" pattern: a single
//! `MDIParentFrame` hosts an `MDICLIENT` common control which in
//! turn hosts several `MDIChildFrame` windows. The user can cascade /
//! tile / minimise / restore the children from a "Window" menu
//! (the menu wiring is left to the caller — we only provide the
//! `cascade_children`, `tile_children`, `close_all_children` and
//! `activate_child` helpers).
//!
//! # Win32 model
//!
//! The parent is a regular top-level window whose single child is a
//! `MDICLIENT` (`WC_MDICLIENT` = `"MDICLIENT"`). Each child is created
//! with `WS_EX_MDICHILD` and made a child of the `MDICLIENT` (not of
//! the parent frame). Children receive `WM_MDIACTIVATE` when the user
//! switches focus between them.
//!
//! # Cross-platform behaviour
//!
//! The constructors are reachable on every platform; on non-Windows
//! they return a struct that stores no real windows. The
//! `hwnd_*` accessors are `#[cfg]`-gated to Windows.

use std::cell::RefCell;
use std::rc::Rc;

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
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ─── Win32 constants ───────────────────────────────────────────────────

#[cfg(target_os = "windows")]
const MDIS_ALLCHILDSTYLES: u32 = 0x0001;

#[cfg(target_os = "windows")]
const WS_EX_MDICHILD: u32 = 0x0000_0040;

// MDI messages sent to MDICLIENT.
#[cfg(target_os = "windows")]
const WM_MDICREATE: u32 = 0x0220;
#[cfg(target_os = "windows")]
const WM_MDIDESTROY: u32 = 0x0221;
#[cfg(target_os = "windows")]
const WM_MDIACTIVATE: u32 = 0x0222;
#[cfg(target_os = "windows")]
const WM_MDIRESTORE: u32 = 0x0223;
#[cfg(target_os = "windows")]
const WM_MDIMAXIMIZE: u32 = 0x0225;
#[cfg(target_os = "windows")]
const WM_MDICASCADE: u32 = 0x0227;
#[cfg(target_os = "windows")]
const WM_MDIICONARRANGE: u32 = 0x0228;
#[cfg(target_os = "windows")]
const WM_MDITILE: u32 = 0x0226;
#[cfg(target_os = "windows")]
const WM_MDIGETACTIVE: u32 = 0x0229;

// Tile / cascade styles.
#[cfg(target_os = "windows")]
const MDITILE_VERTICAL: u32 = 0x0000;
#[cfg(target_os = "windows")]
const MDITILE_HORIZONTAL: u32 = 0x0001;
#[cfg(target_os = "windows")]
const MDITILE_SKIPDISABLED: u32 = 0x0002;
#[cfg(target_os = "windows")]
const MDITILE_ZORDER: u32 = 0x0004;

#[cfg(target_os = "windows")]
const MDICLIENT_CLASS: &str = "RuWxMDIClientClass";
#[cfg(target_os = "windows")]
const MDICHILD_CLASS: &str = "RuWxMDIChildClass";

// ─── CLIENTCREATE / MDICREATESTRUCT ─────────────────────────────────────

/// `CLIENTCREATESTRUCT` passed to `CreateWindowExW` for the
/// `MDICLIENT` window. We keep it as a Win32-only local struct so we
/// do not depend on the `windows-sys` version of the type.
#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case, clippy::upper_case_acronyms)] // mirrors the Win32 header name
struct CLIENTCREATESTRUCT {
    hWindowMenu: isize,
    idFirstChild: u32,
}

/// `MDICREATESTRUCT` passed to `WM_MDICREATE`. We use it to define a
/// child window.
#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case, clippy::upper_case_acronyms)] // mirrors the Win32 header name
struct MDICREATESTRUCTW {
    szClass: *const u16,
    szTitle: *const u16,
    hOwner: isize,
    x: i32,
    y: i32,
    cx: i32,
    cy: i32,
    style: u32,
    lParam: isize,
}

// ─── MDIParentFrame ────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
struct MDIParentInner {
    parent_hwnd: HWND,
    mdi_client_hwnd: HWND,
    /// Monotonic counter for child window ids (used to compute
    /// `idFirstChild` for the `CLIENTCREATESTRUCT`).
    next_child_id: u32,
    /// Live `MDIChildFrame` clones we created; kept so the parent
    /// can enumerate them for `cascade_children` / `tile_children` /
    /// `close_all_children`.
    children: Vec<MDIChildFrame>,
}

#[cfg(not(target_os = "windows"))]
struct MDIParentInner {
    children: Vec<MDIChildFrame>,
}

/// The MDI parent frame: a top-level window that hosts an `MDICLIENT`
/// in its client area.
///
/// Sibling of `wxMDIParentFrame` from wxWidgets. Use it as a normal
/// [`Frame`] replacement: you add widgets to the *parent*, but the
/// child windows are managed by the `MDICLIENT`. Add children with
/// [`MDIParentFrame::add_child`].
#[derive(Clone)]
pub struct MDIParentFrame {
    inner: Rc<RefCell<MDIParentInner>>,
}

impl MDIParentFrame {
    /// Build a new MDI parent. The parent is sized to the given
    /// `width` x `height`, hosts an `MDICLIENT` that fills its
    /// client area, and is owned by `parent` (or owner-less if
    /// `parent` is `None`).
    #[cfg(target_os = "windows")]
    pub fn new(parent: Option<&Frame>, title: &str, width: u32, height: u32) -> Self {
        unsafe { register_mdi_client_class_once() };
        let parent_hwnd = parent.map(|f| f.hwnd()).unwrap_or(std::ptr::null_mut());
        let (parent_hwnd, mdi_client_hwnd) = unsafe {
            let class_name = to_wide("RuWxMDIParentClass");
            let title_wide = to_wide(title);
            let hinstance = GetModuleHandleW(std::ptr::null());
            let parent = CreateWindowExW(
                0,
                class_name.as_ptr(),
                title_wide.as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                width as i32,
                height as i32,
                parent_hwnd,
                std::ptr::null_mut(),
                hinstance,
                std::ptr::null_mut(),
            );
            // Build the `CLIENTCREATESTRUCT` and create the
            // `MDICLIENT` as a child of the parent. We use id 1000
            // for the first child id; the kernel does not require
            // any specific value but reserves 0..0x3FFF for the
            // built-in window ids.
            let mut cc = CLIENTCREATESTRUCT {
                hWindowMenu: 0,
                idFirstChild: 1000,
            };
            let client = CreateWindowExW(
                0,
                to_wide("MDICLIENT").as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | WS_VSCROLL | WS_HSCROLL | MDIS_ALLCHILDSTYLES,
                0,
                0,
                width as i32,
                height as i32,
                parent,
                std::ptr::null_mut(),
                hinstance,
                &mut cc as *mut CLIENTCREATESTRUCT as *mut _,
            );
            crate::platform::window_icon::apply_to_hwnd(parent, None);
            (parent, client)
        };
        Self {
            inner: Rc::new(RefCell::new(MDIParentInner {
                parent_hwnd,
                mdi_client_hwnd,
                next_child_id: 1000,
                children: Vec::new(),
            })),
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn new(_parent: Option<&Frame>, _title: &str, _width: u32, _height: u32) -> Self {
        Self {
            inner: Rc::new(RefCell::new(MDIParentInner {
                children: Vec::new(),
            })),
        }
    }

    /// The MDI parent's `HWND`. `None` on non-Windows.
    #[cfg(target_os = "windows")]
    pub fn parent_hwnd(&self) -> HWND {
        self.inner.borrow().parent_hwnd
    }

    /// The `MDICLIENT` `HWND`. `None` on non-Windows.
    #[cfg(target_os = "windows")]
    pub fn mdi_client_hwnd(&self) -> HWND {
        self.inner.borrow().mdi_client_hwnd
    }

    /// Add a new MDI child with the given title and (initial)
    /// dimensions. The child is created in the `MDICLIENT`, sized to
    /// the requested rectangle (in *MDI client coordinates*, which
    /// are just the parent's client coordinates minus the menu /
    /// toolbar), and returned to the caller.
    #[cfg(target_os = "windows")]
    pub fn add_child(&self, title: &str, x: i32, y: i32, w: i32, h: i32) -> MDIChildFrame {
        unsafe { register_mdi_child_class_once() };
        let (child_hwnd, _mcs) = unsafe {
            let mdi = self.inner.borrow().mdi_client_hwnd;
            let hinstance = GetModuleHandleW(std::ptr::null());
            // The `MDICREATESTRUCTW` class must point at a registered
            // class. We re-use a single class for all children; the
            // Win32 `MDICLIENT` will size and position each child
            // based on `x/y/cx/cy`.
            let class_name = to_wide(MDICHILD_CLASS);
            let title_wide = to_wide(title);
            let mut mcs = MDICREATESTRUCTW {
                szClass: class_name.as_ptr(),
                szTitle: title_wide.as_ptr(),
                hOwner: hinstance as isize,
                x,
                y,
                cx: w,
                cy: h,
                style: WS_CHILD | WS_VISIBLE | WS_CAPTION | WS_THICKFRAME
                    | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX,
                lParam: 0,
            };
            // WM_MDICREATE returns the new child's HWND in the
            // LRESULT of the message (cast to isize).
            let result = SendMessageW(
                mdi,
                WM_MDICREATE,
                0,
                &mut mcs as *mut MDICREATESTRUCTW as isize,
            );
            (result as HWND, mcs)
        };
        let id = self.inner.borrow().next_child_id;
        self.inner.borrow_mut().next_child_id += 1;
        let child = MDIChildFrame {
            #[cfg(target_os = "windows")]
            hwnd: child_hwnd,
            #[cfg(target_os = "windows")]
            _phantom: std::marker::PhantomData,
            #[cfg(not(target_os = "windows"))]
            _placeholder: (),
            title: title.to_string(),
            id,
        };
        // Hold a clone so the parent can enumerate its children
        // for cascade / tile / close-all.
        self.inner.borrow_mut().children.push(child.clone());
        child
    }

    #[cfg(not(target_os = "windows"))]
    pub fn add_child(&self, title: &str, _x: i32, _y: i32, _w: i32, _h: i32) -> MDIChildFrame {
        let id = 0;
        let child = MDIChildFrame {
            _placeholder: (),
            title: title.to_string(),
            id,
        };
        self.inner.borrow_mut().children.push(child.clone());
        child
    }

    /// Cascade all children.
    #[cfg(target_os = "windows")]
    pub fn cascade_children(&self) {
        let mdi = self.inner.borrow().mdi_client_hwnd;
        if mdi.is_null() {
            return;
        }
        // SAFETY: own MDI client.
        unsafe {
            SendMessageW(mdi, WM_MDICASCADE, 0, 0);
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn cascade_children(&self) {}

    /// Tile all children. `horizontal` = `true` produces horizontal
    /// strips; `false` (the default for "tile") produces vertical
    /// strips (i.e. side-by-side columns), which is what most apps
    /// use for "tile".
    #[cfg(target_os = "windows")]
    pub fn tile_children(&self, horizontal: bool) {
        let mdi = self.inner.borrow().mdi_client_hwnd;
        if mdi.is_null() {
            return;
        }
        let flags = if horizontal {
            MDITILE_HORIZONTAL | MDITILE_SKIPDISABLED
        } else {
            MDITILE_VERTICAL | MDITILE_SKIPDISABLED
        };
        // SAFETY: own MDI client.
        unsafe {
            SendMessageW(mdi, WM_MDITILE, flags as usize, 0);
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn tile_children(&self, _horizontal: bool) {}

    /// Close every child window.
    #[cfg(target_os = "windows")]
    pub fn close_all_children(&self) {
        let mdi = self.inner.borrow().mdi_client_hwnd;
        if mdi.is_null() {
            return;
        }
        // `WM_MDIGETACTIVE` returns the currently-active child; we
        // walk the children by repeatedly asking for the next
        // active child (closing the current one each time). This is
        // the documented "iterate over MDI children" pattern.
        loop {
            // SAFETY: own MDI client.
            let active = unsafe { SendMessageW(mdi, WM_MDIGETACTIVE, 0, 0) as HWND };
            if active.is_null() {
                break;
            }
            // SAFETY: own MDI client.
            unsafe {
                SendMessageW(mdi, WM_MDIDESTROY, active as usize, 0);
            }
        }
        self.inner.borrow_mut().children.clear();
    }

    #[cfg(not(target_os = "windows"))]
    pub fn close_all_children(&self) {
        self.inner.borrow_mut().children.clear();
    }

    /// Show the parent window and enter the message loop.
    #[cfg(target_os = "windows")]
    pub fn show(self) {
        let hwnd = self.inner.borrow().parent_hwnd;
        if hwnd.is_null() {
            return;
        }
        // SAFETY: own HWND.
        unsafe {
            ShowWindow(hwnd, SW_SHOW);
            UpdateWindow(hwnd);
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                // Translate MDICLIENT accelerators (e.g. Ctrl+F6 to
                // cycle through children) by letting the MDICLIENT
                // process the message first via `TranslateMDISysAccel`.
                if !msg.hwnd.is_null() {
                    let mdi = self.inner.borrow().mdi_client_hwnd;
                    if !mdi.is_null() {
                        // SAFETY: We are inside the message loop; `mdi`
                        // is a live MDICLIENT HWND and the message
                        // belongs to this thread.
                        if windows_sys::Win32::UI::WindowsAndMessaging::TranslateMDISysAccel(
                            mdi,
                            &msg,
                        ) != 0
                        {
                            continue;
                        }
                    }
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn show(self) {}

    /// The number of live children.
    pub fn child_count(&self) -> usize {
        self.inner.borrow().children.len()
    }

    /// Destroy the MDI parent window and clear child handles.
    #[cfg(target_os = "windows")]
    pub fn destroy(&self) {
        let hwnd = self.inner.borrow().parent_hwnd;
        if !hwnd.is_null() {
            // SAFETY: `hwnd` is the top-level MDI parent we created in `new`.
            unsafe {
                DestroyWindow(hwnd);
            }
            let mut inner = self.inner.borrow_mut();
            inner.parent_hwnd = std::ptr::null_mut();
            inner.mdi_client_hwnd = std::ptr::null_mut();
            inner.children.clear();
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn destroy(&self) {
        self.inner.borrow_mut().children.clear();
    }
}

// ─── MDIChildFrame ─────────────────────────────────────────────────────

/// A child of an [`MDIParentFrame`]. Built via
/// [`MDIParentFrame::add_child`]. Children can be activated,
/// maximised, restored and closed via the parent.
#[derive(Clone)]
pub struct MDIChildFrame {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(target_os = "windows")]
    _phantom: std::marker::PhantomData<*mut ()>,
    #[cfg(not(target_os = "windows"))]
    _placeholder: (),
    title: String,
    id: u32,
}

impl MDIChildFrame {
    /// Native window handle. `None` on non-Windows.
    #[cfg(target_os = "windows")]
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// Maximise this child.
    #[cfg(target_os = "windows")]
    pub fn maximize(&self) {
        let parent = self.parent_mdi();
        if let Some(mdi) = parent {
            // SAFETY: own MDI client; the lparam is a HWND.
            unsafe {
                SendMessageW(mdi, WM_MDIMAXIMIZE, self.hwnd as usize, 0);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn maximize(&self) {}

    /// Restore a maximised / minimised child.
    #[cfg(target_os = "windows")]
    pub fn restore(&self) {
        let parent = self.parent_mdi();
        if let Some(mdi) = parent {
            // SAFETY: own MDI client; the lparam is a HWND.
            unsafe {
                SendMessageW(mdi, WM_MDIRESTORE, self.hwnd as usize, 0);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn restore(&self) {}

    /// Activate (bring to focus) this child.
    #[cfg(target_os = "windows")]
    pub fn activate(&self) {
        let parent = self.parent_mdi();
        if let Some(mdi) = parent {
            // SAFETY: own MDI client; the lparam is a HWND.
            unsafe {
                SendMessageW(mdi, WM_MDIACTIVATE, self.hwnd as usize, 0);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn activate(&self) {}

    /// The id assigned by the parent when this child was created.
    pub fn id(&self) -> u32 {
        self.id
    }

    /// The title used to create this child.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Look up the parent `MDICLIENT` HWND. We do this by walking
    /// `GetParent` once, but for now we keep it private and trust
    /// the parent to forward `activate` / `maximize` calls.
    #[cfg(target_os = "windows")]
    fn parent_mdi(&self) -> Option<HWND> {
        // SAFETY: `GetParent` returns the parent HWND; for an MDI
        // child this is the MDICLIENT window.
        let parent = unsafe { GetParent(self.hwnd) };
        if parent.is_null() {
            None
        } else {
            Some(parent)
        }
    }
}

// ─── Win32 helpers (private) ───────────────────────────────────────────

/// Idempotently register the `MDICLIENT` host window class. The
/// `MDICLIENT` *itself* is registered as a built-in window class by
/// `comctl32`; we only need to register the wrapper parent.
#[cfg(target_os = "windows")]
unsafe fn register_mdi_client_class_once() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_name = to_wide("RuWxMDIParentClass");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(mdi_parent_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: crate::platform::window_icon::class_icons().0,
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: (COLOR_APPWORKSPACE + 1) as usize as HBRUSH,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: crate::platform::window_icon::class_icons().1,
        };
        RegisterClassExW(&wc);
        // Also register the MDICHILD class. We could register it
        // inside the `add_child` Once, but registering it together
        // with the parent keeps the registrations in one place.
        let child_class = to_wide(MDICHILD_CLASS);
        let child_wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(mdi_child_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: crate::platform::window_icon::class_icons().0,
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: (COLOR_WINDOW + 1) as usize as HBRUSH,
            lpszMenuName: std::ptr::null(),
            lpszClassName: child_class.as_ptr(),
            hIconSm: crate::platform::window_icon::class_icons().1,
        };
        RegisterClassExW(&child_wc);
    });
}

#[cfg(target_os = "windows")]
unsafe fn register_mdi_child_class_once() {
    // The class is registered as part of `register_mdi_client_class_once`
    // (called by the parent constructor). Re-calling the registration
    // here is a no-op because of the `Once` inside that helper.
    register_mdi_client_class_once();
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn mdi_parent_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // MDI parents receive a `WM_MDIACTIVATE` echo; the default
    // behaviour is to forward it to the activated child, which is
    // exactly what we want. We let the default handler do that
    // (i.e. return its LRESULT).
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn mdi_child_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // MDI child default WndProc handles close / activation
    // automatically. We could intercept `WM_CLOSE` to ask the user
    // to save first, but that's left to a higher-level wrapper.
    DefWindowProcW(hwnd, msg, wparam, lparam)
}
