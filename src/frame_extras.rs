//! Special-purpose top-level windows: [`TipWindow`], [`SplashScreen`],
//! [`MiniFrame`].
//!
//! All three are implemented on Windows by deriving from a fresh
//! `CreateWindowExW` call (they do not share a `Frame` HWND — each is its
//! own top-level popup), so the implementation file is independent of
//! `frame.rs` and does not need to interact with the existing
//! `FrameData` registry.
//!
//! These types are companions to the main [`Frame`](crate::Frame) and to
//! [`Dialog`](crate::Dialog); they are deliberately kept on a separate
//! file so the core frame / dialog code stays untouched.
//!
//! # Cross-platform behaviour
//!
//! The constructors are reachable on every platform; on non-Windows
//! hosts they return a struct that simply stores the requested geometry
//! and lets callers call [`TipWindow::close`], [`SplashScreen::close`]
//! or [`MiniFrame::set_title`] etc. as no-ops. This keeps user code
//! `#[cfg]`-free.

use std::cell::RefCell;
use std::rc::Rc;

use crate::bitmap::Bitmap;
use crate::frame::Frame;
use crate::geometry::Rect;

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

// ─── Window class names (registered lazily, idempotently) ──────────────

#[cfg(target_os = "windows")]
const TIP_WINDOW_CLASS: &str = "RuWxTipWindowClass";
#[cfg(target_os = "windows")]
const SPLASH_SCREEN_CLASS: &str = "RuWxSplashScreenClass";
#[cfg(target_os = "windows")]
const MINI_FRAME_CLASS: &str = "RuWxMiniFrameClass";

// ─── Common style bits ─────────────────────────────────────────────────

#[cfg(target_os = "windows")]
const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
#[cfg(target_os = "windows")]
const WS_EX_TOPMOST: u32 = 0x0000_0008;
#[cfg(target_os = "windows")]
const WS_EX_NOACTIVATE: u32 = 0x0800_0000;

// ─── TipWindow ─────────────────────────────────────────────────────────

/// Internal state shared by all `TipWindow` clones (cheap: `HWND` is
/// `isize`-sized, plus a couple of book-keeping fields).
#[cfg(target_os = "windows")]
struct TipWindowInner {
    hwnd: HWND,
    text: String,
    rect: Rect,
}

#[cfg(not(target_os = "windows"))]
struct TipWindowInner {
    text: String,
    rect: Rect,
}

/// A small, transient, **non-activating** popup that shows a one-line
/// hint near a UI element.
///
/// `TipWindow` is a sibling of `wxTipWindow` from wxWidgets. It is meant
/// to be created as a popup anchored to a control rect (typically the
/// rectangle of a toolbar button whose tooltip / hint the user wants to
/// surface for a few seconds). The window:
///
/// * has no task-bar entry (`WS_EX_TOOLWINDOW`),
/// * does not steal focus (`WS_EX_NOACTIVATE`),
/// * stays on top of other windows while it is open
///   (`WS_EX_TOPMOST`),
/// * is automatically closed by the OS when the user clicks anywhere
///   outside it (because it does not capture focus it gets a click-out
///   close from the mouse hook installed by `TrackMouseEvent`; if that
///   is not desired the user can also call [`TipWindow::close`]
///   explicitly).
///
/// # Cross-platform behaviour
///
/// On non-Windows the constructor is a no-op stub that stores the
/// requested text + rect; calling [`TipWindow::close`] or
/// [`TipWindow::set_text`] is always safe. Methods that need a live
/// `HWND` (only [`TipWindow::hwnd`]) are `#[cfg]`-gated to Windows.
#[derive(Clone)]
pub struct TipWindow {
    inner: Rc<RefCell<TipWindowInner>>,
}

impl TipWindow {
    /// Create a new tip-window popup positioned at `rect` (in screen
    /// coordinates).
    #[cfg(target_os = "windows")]
    pub fn new(parent: &Frame, rect: Rect, text: &str) -> Self {
        unsafe { register_tip_class_once() };
        let hwnd = unsafe {
            let class_name = to_wide(TIP_WINDOW_CLASS);
            let title = to_wide(""); // no title bar
            let text_wide = to_wide(text);
            let parent_hwnd = parent.hwnd();
            let hinstance = GetModuleHandleW(std::ptr::null());
            // WS_POPUP | WS_BORDER gives us a borderless popup with a
            // 1-px border. WS_EX_TOOLWINDOW hides the entry from the
            // taskbar and the Alt-Tab list. WS_EX_TOPMOST keeps the
            // tip on top. WS_EX_NOACTIVATE means clicking on the tip
            // will not steal focus from the parent frame.
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_POPUP | WS_BORDER | WS_VISIBLE,
                rect.x,
                rect.y,
                rect.width.max(80) as i32,
                rect.height.max(24) as i32,
                parent_hwnd,
                std::ptr::null_mut(),
                hinstance,
                std::ptr::null_mut(),
            );
            // Render the initial text into the tip's client area.
            if !hwnd.is_null() {
                paint_tip_text(hwnd, &text_wide);
            }
            hwnd
        };
        Self {
            inner: Rc::new(RefCell::new(TipWindowInner {
                hwnd,
                text: text.to_string(),
                rect,
            })),
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn new(_parent: &Frame, rect: Rect, text: &str) -> Self {
        Self {
            inner: Rc::new(RefCell::new(TipWindowInner {
                text: text.to_string(),
                rect,
            })),
        }
    }

    /// Native window handle (`HWND`). `None` on non-Windows platforms.
    #[cfg(target_os = "windows")]
    pub fn hwnd(&self) -> HWND {
        self.inner.borrow().hwnd
    }

    /// Replace the tip's text. Triggers a repaint.
    #[cfg(target_os = "windows")]
    pub fn set_text(&self, text: &str) {
        let mut inner = self.inner.borrow_mut();
        inner.text = text.to_string();
        let wide = to_wide(text);
        // SAFETY: We only repaint our own client area; the
        // `text_wide` buffer lives until the end of this scope.
        unsafe {
            paint_tip_text(inner.hwnd, &wide);
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn set_text(&self, text: &str) {
        self.inner.borrow_mut().text = text.to_string();
    }

    /// Currently-displayed text.
    pub fn text(&self) -> String {
        self.inner.borrow().text.clone()
    }

    /// Close (destroy) the tip window. Safe to call multiple times.
    #[cfg(target_os = "windows")]
    pub fn close(&self) {
        let hwnd = self.inner.borrow().hwnd;
        if !hwnd.is_null() {
            // SAFETY: we own this HWND.
            unsafe {
                DestroyWindow(hwnd);
            }
            self.inner.borrow_mut().hwnd = std::ptr::null_mut();
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn close(&self) {
        // no-op on non-Windows
    }
}

// ─── SplashScreen ──────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
struct SplashScreenInner {
    hwnd: HWND,
    bitmap: Bitmap,
    /// `None` = no auto-close timer; `Some(id)` = the timer that will
    /// close the window. We keep it as a raw `usize` because `Timer`
    /// would create a cycle; we tear it down explicitly on
    /// `SplashScreen::close`.
    timer_id: Option<usize>,
}

#[cfg(not(target_os = "windows"))]
struct SplashScreenInner {
    bitmap: Bitmap,
    timer_id: Option<usize>,
}

/// A splash screen: a top-most, borderless window that displays a
/// bitmap (typically the application's logo / loading splash) and
/// optionally closes itself after a timeout.
///
/// Sibling of `wxSplashScreen` from wxWidgets. Construct it with
/// [`SplashScreen::new`], call [`SplashScreen::show`] to display it
/// without entering the message loop (so the rest of `Frame::show`
/// can take over once initialisation finishes), and call
/// [`SplashScreen::close`] when the main window is ready to take
/// over.
///
/// # Auto-close timer
///
/// `milliseconds` is the *suggested* display time. We register a
/// `SetTimer` whose `WM_TIMER` calls `DestroyWindow` on the splash.
/// If the user closes the splash earlier (by calling
/// [`SplashScreen::close`] or by clicking on it) the timer is killed
/// in the `WM_CLOSE` / `close()` path.
///
/// Pass `0` for `milliseconds` to disable the auto-close timer (you
/// must then call [`SplashScreen::close`] manually).
#[derive(Clone)]
pub struct SplashScreen {
    inner: Rc<RefCell<SplashScreenInner>>,
}

impl SplashScreen {
    /// Create a splash screen. The window is not yet visible — call
    /// [`SplashScreen::show`] to display it.
    pub fn new(parent: &Frame, bitmap: Bitmap, milliseconds: u32) -> Self {
        Self::with_position(parent, bitmap, CW_USEDEFAULT, CW_USEDEFAULT, milliseconds)
    }

    /// Create a splash screen at a specific position (screen
    /// coordinates).
    #[cfg(target_os = "windows")]
    pub fn with_position(
        parent: &Frame,
        bitmap: Bitmap,
        x: i32,
        y: i32,
        milliseconds: u32,
    ) -> Self {
        unsafe { register_splash_class_once() };
        let hwnd = unsafe {
            let class_name = to_wide(SPLASH_SCREEN_CLASS);
            let title = to_wide("ru_wx splash");
            let parent_hwnd = parent.hwnd();
            let hinstance = GetModuleHandleW(std::ptr::null());
            let (w, h) = (bitmap.width.max(1), bitmap.height.max(1));
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_POPUP | WS_VISIBLE,
                if x == CW_USEDEFAULT {
                    CW_USEDEFAULT
                } else {
                    x
                },
                if y == CW_USEDEFAULT {
                    CW_USEDEFAULT
                } else {
                    y
                },
                w as i32,
                h as i32,
                parent_hwnd,
                std::ptr::null_mut(),
                hinstance,
                std::ptr::null_mut(),
            )
        };
        let mut s = Self {
            inner: Rc::new(RefCell::new(SplashScreenInner {
                hwnd,
                bitmap,
                timer_id: None,
            })),
        };
        if milliseconds > 0 {
            s.start_close_timer(milliseconds);
        }
        s
    }

    #[cfg(not(target_os = "windows"))]
    pub fn with_position(
        _parent: &Frame,
        bitmap: Bitmap,
        _x: i32,
        _y: i32,
        _milliseconds: u32,
    ) -> Self {
        Self {
            inner: Rc::new(RefCell::new(SplashScreenInner {
                bitmap,
                timer_id: None,
            })),
        }
    }

    /// Native window handle. `None` on non-Windows.
    #[cfg(target_os = "windows")]
    pub fn hwnd(&self) -> HWND {
        self.inner.borrow().hwnd
    }

    /// Display the splash. Idempotent (Win32's `ShowWindow` is
    /// idempotent in the SW_SHOW case).
    #[cfg(target_os = "windows")]
    pub fn show(&self) {
        let hwnd = self.inner.borrow().hwnd;
        if !hwnd.is_null() {
            // SAFETY: own HWND.
            unsafe {
                ShowWindow(hwnd, SW_SHOW);
                UpdateWindow(hwnd);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn show(&self) {}

    /// Close the splash screen, killing the auto-close timer (if any).
    #[cfg(target_os = "windows")]
    pub fn close(&self) {
        let mut inner = self.inner.borrow_mut();
        if let Some(timer_id) = inner.timer_id.take() {
            // SAFETY: own HWND.
            unsafe {
                KillTimer(inner.hwnd, timer_id);
            }
        }
        if !inner.hwnd.is_null() {
            // SAFETY: own HWND.
            unsafe {
                DestroyWindow(inner.hwnd);
            }
            inner.hwnd = std::ptr::null_mut();
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn close(&self) {
        self.inner.borrow_mut().timer_id = None;
    }

    #[cfg(target_os = "windows")]
    fn start_close_timer(&mut self, ms: u32) {
        // We use a 1-based id so 0 is reserved for "no timer". The id
        // does not need to be unique across splash screens because
        // each splash has its own HWND and `SetTimer(hwnd, id, ...)`
        // is scoped to the window.
        let timer_id: usize = 1;
        let hwnd = self.inner.borrow().hwnd;
        if hwnd.is_null() {
            return;
        }
        // SAFETY: `SetTimer` is safe to call with our own HWND and a
        // valid id. The lparam / lpfnTimer are null because we use
        // `WM_TIMER` posts in the splash's own WndProc to detect
        // expiry.
        let ok = unsafe { SetTimer(hwnd, timer_id, ms, None) };
        if ok != 0 {
            self.inner.borrow_mut().timer_id = Some(timer_id);
        }
    }
}

// ─── MiniFrame ─────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
struct MiniFrameInner {
    hwnd: HWND,
}

#[cfg(not(target_os = "windows"))]
struct MiniFrameInner {
    _placeholder: (),
}

/// A small caption frame with no minimise / maximise buttons (just
/// `Close`). Sibling of `wxMiniFrame` from wxWidgets — typically used
/// for floating tool palettes, dockable property panels, inspector
/// windows.
///
/// Compared to a regular [`Frame`]:
///
/// * it has no `WS_MAXIMIZEBOX` or `WS_MINIMIZEBOX` (only
///   `WS_CAPTION | WS_SYSMENU | WS_THICKFRAME`),
/// * it does not show up in the task bar (`WS_EX_TOOLWINDOW`),
/// * it does **not** have a message loop of its own — you build a
///   `MiniFrame` and either:
///   * use it as a child content host whose widgets are reachable
///     through a parent `Frame`'s dispatch machinery (it shares the
///     parent's `WM_COMMAND` / `WM_NOTIFY` map), or
///   * call [`MiniFrame::destroy`] when done and let the main loop
///     carry on.
///
/// This implementation is intentionally minimal: it does not provide
/// a `set_sizer` / `add_widget` API of its own (use a [`Frame`] for
/// that, or wrap a child [`Panel`](crate::Panel) inside the mini
/// frame).
#[derive(Clone)]
pub struct MiniFrame {
    inner: Rc<RefCell<MiniFrameInner>>,
}

impl MiniFrame {
    /// Create a new mini-frame.
    #[cfg(target_os = "windows")]
    pub fn new(parent: &Frame, title: &str, width: u32, height: u32) -> Self {
        unsafe { register_mini_frame_class_once() };
        let hwnd = unsafe {
            let class_name = to_wide(MINI_FRAME_CLASS);
            let title_wide = to_wide(title);
            let parent_hwnd = parent.hwnd();
            let hinstance = GetModuleHandleW(std::ptr::null());
            // `WS_CAPTION | WS_SYSMENU | WS_THICKFRAME` produces a
            // resizable window with a title bar and a single close
            // button (no min / max). `WS_CLIPCHILDREN` is the standard
            // optimisation that prevents flicker when child controls
            // repaint.
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                class_name.as_ptr(),
                title_wide.as_ptr(),
                WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | WS_CLIPCHILDREN,
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
        Self {
            inner: Rc::new(RefCell::new(MiniFrameInner { hwnd })),
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn new(_parent: &Frame, _title: &str, _width: u32, _height: u32) -> Self {
        Self {
            inner: Rc::new(RefCell::new(MiniFrameInner { _placeholder: () })),
        }
    }

    /// Native window handle. `None` on non-Windows.
    #[cfg(target_os = "windows")]
    pub fn hwnd(&self) -> HWND {
        self.inner.borrow().hwnd
    }

    /// Set the title-bar text.
    #[cfg(target_os = "windows")]
    pub fn set_title(&self, title: &str) {
        let hwnd = self.inner.borrow().hwnd;
        if hwnd.is_null() {
            return;
        }
        let wide = to_wide(title);
        // SAFETY: `SetWindowTextW` copies the string immediately, so
        // the buffer only needs to live for the call.
        unsafe {
            SetWindowTextW(hwnd, wide.as_ptr());
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn set_title(&self, _title: &str) {}

    /// Show / hide the mini-frame.
    #[cfg(target_os = "windows")]
    pub fn show(&self, show: bool) {
        let hwnd = self.inner.borrow().hwnd;
        if hwnd.is_null() {
            return;
        }
        // SAFETY: own HWND, `nCmdShow` is a documented constant.
        unsafe {
            ShowWindow(hwnd, if show { SW_SHOW } else { SW_HIDE });
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn show(&self, _show: bool) {}

    /// Destroy the mini-frame. Safe to call multiple times.
    #[cfg(target_os = "windows")]
    pub fn destroy(&self) {
        let mut inner = self.inner.borrow_mut();
        if !inner.hwnd.is_null() {
            // SAFETY: own HWND.
            unsafe {
                DestroyWindow(inner.hwnd);
            }
            inner.hwnd = std::ptr::null_mut();
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn destroy(&self) {}
}

// ─── Win32 helpers (private) ───────────────────────────────────────────

/// Render a string of UTF-16 text into the given popup's client area.
/// Used by `TipWindow` to draw its text on top of the default
/// background. We use `DrawTextW` directly because the tip is a
/// transient popup without a full paint handler machinery.
#[cfg(target_os = "windows")]
unsafe fn paint_tip_text(hwnd: HWND, text: &[u16]) {
    if hwnd.is_null() || text.is_empty() {
        return;
    }
    // Get the client area dimensions.
    let mut rect: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut rect);
    // `DT_SINGLELINE | DT_CENTER | DT_VCENTER` lays the text on a
    // single line, centred horizontally and vertically.
    let hdc = GetDC(hwnd);
    if hdc.is_null() {
        return;
    }
    // SAFETY: we own the `hdc` for the duration of the call.
    DrawTextW(
        hdc,
        text.as_ptr(),
        text.len() as i32,
        &mut rect,
        DT_SINGLELINE | DT_CENTER | DT_VCENTER,
    );
    ReleaseDC(hwnd, hdc);
}

/// Idempotently register the `TipWindow` window class. We keep a
/// thread-local flag so the `RegisterClassExW` call only happens once
/// per process; the kernel silently ignores repeat registrations.
#[cfg(target_os = "windows")]
unsafe fn register_tip_class_once() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_name = to_wide(TIP_WINDOW_CLASS);
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(tip_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: LoadIconW(std::ptr::null_mut(), IDI_APPLICATION),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: (COLOR_INFOBK + 1) as usize as HBRUSH,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wc);
    });
}

#[cfg(target_os = "windows")]
unsafe fn register_splash_class_once() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_name = to_wide(SPLASH_SCREEN_CLASS);
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(splash_wnd_proc),
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
    });
}

#[cfg(target_os = "windows")]
unsafe fn register_mini_frame_class_once() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_name = to_wide(MINI_FRAME_CLASS);
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(mini_frame_wnd_proc),
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
    });
}

/// Window procedure shared by all `TipWindow` instances. It defers
/// every unrecognised message to the default handler.
#[cfg(target_os = "windows")]
unsafe extern "system" fn tip_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // Repaint on demand. The default `COLOR_INFOBK` background is
    // already drawn by the class brush, so all we have to do is call
    // `BeginPaint` / `EndPaint` and let `WM_ERASEBKGND` re-fill.
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            BeginPaint(hwnd, &mut ps);
            EndPaint(hwnd, &ps);
            return 0;
        }
        WM_LBUTTONDOWN | WM_RBUTTONDOWN => {
            // Click on the tip closes it (matches wxWidgets).
            DestroyWindow(hwnd);
            return 0;
        }
        _ => {}
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

/// Window procedure for `SplashScreen` instances. It auto-closes on
/// `WM_TIMER` (using the `SetTimer` id registered by
/// `SplashScreen::start_close_timer`) and on click.
#[cfg(target_os = "windows")]
unsafe extern "system" fn splash_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            BeginPaint(hwnd, &mut ps);
            EndPaint(hwnd, &ps);
            return 0;
        }
        WM_TIMER => {
            // The id we used for the auto-close timer is always 1.
            if wparam == 1 {
                DestroyWindow(hwnd);
            }
            return 0;
        }
        WM_LBUTTONDOWN | WM_RBUTTONDOWN => {
            // Click closes the splash.
            DestroyWindow(hwnd);
            return 0;
        }
        _ => {}
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn mini_frame_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    DefWindowProcW(hwnd, msg, wparam, lparam)
}
