//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Per-widget tooltips.
//!
//! Mirrors `wxToolTip`: a small text bubble shown when the user hovers
//! the mouse over a widget. Tooltips can be attached to any widget via
//! [`ToolTip::attach`] and detached via [`ToolTip::detach`]. A global
//! enable/disable switch is available through [`ToolTip::enable`].
//!
//! # Win32 implementation
//!
//! On Windows, tooltips are implemented as a single `tooltips_class32`
//! child of the top-level window (the class is registered by
//! `comctl32.dll` and is available on every Windows installation).
//! Multiple widgets in the same top-level share that one tooltip
//! control. The library walks `GetAncestor(target, GA_ROOT)` to locate
//! the top-level parent, then looks up (or creates) the tooltip child
//! for it.
//!
//! On non-Windows platforms this is a no-op stub so the API is still
//! available.

/// Win32 implementation of [`ToolTip`] — a private helper
/// module that owns the `tooltips_class32` registration
/// constants, the per-top-level-window tooltip handle cache,
/// and the FFI calls (`CreateWindowExW`, `AddToolW`,
/// `TrackActivate`, etc.) that the public methods on
/// [`ToolTip`] dispatch to. Kept in a `mod imp { }` block so
/// the entire Win32 surface is hidden behind the safe
/// `ToolTip` API and so a future non-Windows backend can
/// provide a sibling `mod imp { }` gated on
/// `#[cfg(not(target_os = "windows"))]`.
#[cfg(target_os = "windows")]
mod imp {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::{Mutex, OnceLock};

    use crate::core::widget::WidgetRef;

    use windows_sys::Win32::Foundation::{HINSTANCE, HWND, RECT};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, FindWindowExW, GetAncestor, SendMessageW, CW_USEDEFAULT,
        GA_ROOT, WM_USER, WS_POPUP,
    };

    use crate::platform::win32::to_wide;

    // ---- Win32 tooltip constants (not all exposed by windows-sys 0.59) ----
    /// Show the tooltip even when the parent window is inactive.
    const TTS_ALWAYSTIP: u32 = 0x0000_0001;
    /// Don't strip `&` from the text (don't interpret as a mnemonic).
    const TTS_NOPREFIX: u32 = 0x0000_0002;
    /// `uId` member of `TOOLINFO` holds an HWND rather than an identifier.
    const TTF_IDISHWND: u32 = 0x0000_0001;
    /// Subclass the tool window to detect mouse events automatically.
    const TTF_SUBCLASS: u32 = 0x0000_0010;

    const TTM_ACTIVATE: u32 = WM_USER + 1; // 0x0401
    // Use the **wide** message variants: `WM_USER + 4/5/12` are the ANSI
    // (`...A`) versions. Sending a `TOOLINFOW` with a wide `lpszText` to
    // the ANSI handler makes the control read the text as ANSI and show
    // only the first character (or nothing at all).
    const TTM_ADDTOOL: u32 = WM_USER + 50; // TTM_ADDTOOLW   (0x0432)
    const TTM_DELTOOL: u32 = WM_USER + 51; // TTM_DELTOOLW   (0x0433)
    const TTM_UPDATETIPTEXT: u32 = WM_USER + 57; // TTM_UPDATETIPTEXTW (0x0439)

    const TOOLTIPS_CLASS: &str = "tooltips_class32";

    /// Local copy of `TOOLINFOW` (`<commctrl.h>`) so we don't depend on
    /// the exact layout/visibility of the version exposed by
    /// `windows-sys` 0.59.
    #[repr(C)]
    #[allow(non_snake_case, dead_code)]
    struct ToolInfoW {
        cb_size: u32,
        u_flags: u32,
        hwnd: HWND,
        u_id: usize,
        rect: RECT,
        h_inst: HINSTANCE,
        lpsz_text: *const u16,
        l_param: isize,
        lp_reserved: *mut core::ffi::c_void,
    }
    const fn toolinfo_size() -> u32 {
        core::mem::size_of::<ToolInfoW>() as u32
    }

    /// Track every tooltip control this library has ever created so
    /// that the global [`ToolTip::enable`] switch can iterate them.
    /// Stored as `usize` (HWNDs are pointer-sized) because the raw
    /// `*mut c_void` is `!Send` and can't live in a `Mutex`.
    static TOOLTIP_HANDLES: OnceLock<Mutex<Vec<usize>>> = OnceLock::new();
    fn tooltip_handles() -> &'static Mutex<Vec<usize>> {
        TOOLTIP_HANDLES.get_or_init(|| Mutex::new(Vec::new()))
    }

    /// The tooltip control stores the `lpszText` **pointer**, not a copy
    /// of the string. Callers typically write
    /// `ToolTip::new(text).attach(widget)` and drop the temporary
    /// immediately, which would free the buffer and leave the control
    /// pointing at freed memory (tooltip shows nothing / garbage). To
    /// keep the text alive for the whole life of the widget we own the
    /// wide buffers here, keyed by target `HWND`.
    static TEXT_STORE: OnceLock<Mutex<std::collections::HashMap<usize, Vec<u16>>>> =
        OnceLock::new();
    fn text_store() -> &'static Mutex<std::collections::HashMap<usize, Vec<u16>>> {
        TEXT_STORE.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
    }

    /// Persist `wide` for `target` and return a stable pointer to the
    /// stored buffer. Replacing an existing entry frees the old buffer,
    /// so callers must update the control with the returned pointer.
    fn store_text(target: HWND, wide: Vec<u16>) -> *const u16 {
        let mut map = text_store().lock().unwrap();
        map.insert(target as usize, wide);
        map.get(&(target as usize)).unwrap().as_ptr()
    }

    fn forget_text(target: HWND) {
        text_store().lock().unwrap().remove(&(target as usize));
    }

    /// Locate the (single) tooltip child of the top-level window
    /// `top`, creating one if needed.
    unsafe fn find_or_create_tooltip_for(top: HWND) -> HWND {
        if top.is_null() {
            return std::ptr::null_mut();
        }
        let wide_class = to_wide(TOOLTIPS_CLASS);
        let existing = FindWindowExW(
            top,
            std::ptr::null_mut(),
            wide_class.as_ptr(),
            std::ptr::null_mut(),
        );
        if !existing.is_null() {
            return existing;
        }
        let hwnd = CreateWindowExW(
            0,
            wide_class.as_ptr(),
            std::ptr::null(),
            WS_POPUP | TTS_ALWAYSTIP | TTS_NOPREFIX,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            top,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        if !hwnd.is_null() {
            tooltip_handles().lock().unwrap().push(hwnd as usize);
        }
        hwnd
    }

    unsafe fn fill_toolinfo(target: HWND, text_ptr: *const u16) -> ToolInfoW {
        ToolInfoW {
            cb_size: toolinfo_size(),
            u_flags: TTF_SUBCLASS | TTF_IDISHWND,
            // Win32 samples set both `hwnd` and `uId` to the tool HWND
            // when `TTF_IDISHWND` is set.
            hwnd: target,
            u_id: target as usize,
            rect: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            h_inst: std::ptr::null_mut(),
            lpsz_text: text_ptr,
            l_param: 0,
            lp_reserved: std::ptr::null_mut(),
        }
    }

    unsafe fn attach_to_hwnd(target: HWND, text_ptr: *const u16) {
        let top = GetAncestor(target, GA_ROOT);
        let tip = find_or_create_tooltip_for(top);
        if tip.is_null() {
            return;
        }
        let info = fill_toolinfo(target, text_ptr);
        // Remove any previous registration for this target so we don't
        // end up with duplicate tool entries.
        SendMessageW(tip, TTM_DELTOOL, 0, &info as *const _ as isize);
        SendMessageW(tip, TTM_ADDTOOL, 0, &info as *const _ as isize);
    }

    unsafe fn update_tip_text(target: HWND, text_ptr: *const u16) {
        let top = GetAncestor(target, GA_ROOT);
        let tip = find_or_create_tooltip_for(top);
        if tip.is_null() {
            return;
        }
        let info = fill_toolinfo(target, text_ptr);
        SendMessageW(tip, TTM_UPDATETIPTEXT, 0, &info as *const _ as isize);
    }

    unsafe fn detach_from_hwnd(target: HWND) {
        let top = GetAncestor(target, GA_ROOT);
        if top.is_null() {
            return;
        }
        let wide_class = to_wide(TOOLTIPS_CLASS);
        let tip = FindWindowExW(
            top,
            std::ptr::null_mut(),
            wide_class.as_ptr(),
            std::ptr::null_mut(),
        );
        if tip.is_null() {
            return;
        }
        let info = fill_toolinfo(target, std::ptr::null());
        SendMessageW(tip, TTM_DELTOOL, 0, &info as *const _ as isize);
    }

    /// Per-widget tooltip.
    #[derive(Clone)]
    pub struct ToolTip {
        inner: Rc<RefCell<ToolTipInner>>,
    }

    struct ToolTipInner {
        text: String,
        target_hwnd: Option<HWND>,
    }

    impl ToolTip {
        /// Create a new tooltip with the given text. Use
        /// [`ToolTip::attach`] to bind it to a widget.
        pub fn new(text: &str) -> Self {
            Self {
                inner: Rc::new(RefCell::new(ToolTipInner {
                    text: text.to_string(),
                    target_hwnd: None,
                })),
            }
        }

        /// Get the current text.
        pub fn text(&self) -> String {
            self.inner.borrow().text.clone()
        }

        /// Update the tooltip text. If the tooltip is currently
        /// attached to a widget, the change is reflected immediately
        /// on screen.
        pub fn set_text(&self, text: &str) {
            let mut inner = self.inner.borrow_mut();
            inner.text = text.to_string();
            if let Some(hwnd) = inner.target_hwnd {
                // The control keeps the pointer, so the buffer has to
                // outlive this `ToolTip`; park it in the global store.
                let ptr = store_text(hwnd, to_wide(text));
                // SAFETY: `ptr` references the global store, valid until
                // the entry is replaced/removed.
                unsafe { update_tip_text(hwnd, ptr) };
            }
        }

        /// Bind this tooltip to a widget. The widget must belong to a
        /// top-level window (`Frame`, `TopLevelWindow`, etc.).
        ///
        /// Calling `attach` again on the same widget replaces the
        /// previous registration, so the new text takes effect on the
        /// next hover.
        ///
        /// Pass the widget's [`WidgetRef`], obtained via
        /// `widget.as_widget_ref()`, to stay decoupled from any
        /// specific widget type.
        pub fn attach(&self, target: &WidgetRef) {
            let hwnd = target.borrow().native_handle() as HWND;
            if hwnd.is_null() {
                return;
            }
            let mut inner = self.inner.borrow_mut();
            inner.target_hwnd = Some(hwnd);
            // The tooltip control stores the `lpszText` pointer rather
            // than copying it, and callers usually drop the `ToolTip`
            // right after `attach`. Keep the buffer alive in the global
            // store so the pointer stays valid for the widget's life.
            let ptr = store_text(hwnd, to_wide(&inner.text));
            // SAFETY: `ptr` references the global store, valid until the
            // entry is replaced/removed via `detach`.
            unsafe { attach_to_hwnd(hwnd, ptr) };
        }

        /// Globally enable or disable tooltip display. Affects every
        /// tooltip control this library has created in the current
        /// process.
        pub fn enable(enabled: bool) {
            let handles = tooltip_handles().lock().unwrap();
            let flag = if enabled { 1 } else { 0 };
            for &hwnd in handles.iter() {
                // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                unsafe {
                    SendMessageW(hwnd as HWND, TTM_ACTIVATE, 0, flag);
                }
            }
        }

        /// Remove any tooltip previously attached to the given widget.
        /// Has no effect if the widget has no tooltip.
        pub fn detach(target: &WidgetRef) {
            let hwnd = target.borrow().native_handle() as HWND;
            if hwnd.is_null() {
                return;
            }
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe { detach_from_hwnd(hwnd) };
            forget_text(hwnd);
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    /// Per-widget tooltip — non-Windows stub. All methods are no-ops
    /// so the API is still available on every platform.
    #[derive(Clone, Debug, Default)]
    pub struct ToolTip {
        text: String,
    }

    impl ToolTip {
        pub fn new(text: &str) -> Self {
            Self {
                text: text.to_string(),
            }
        }
        pub fn text(&self) -> String {
            self.text.clone()
        }
        pub fn set_text(&self, text: &str) {
            // No-op on non-Windows builds; the field is intentionally
            // left in its original state because `&self` matches the
            // Windows impl. Callers should not rely on persistence.
            let _ = text;
        }
        pub fn attach(&self, _target: &crate::core::widget::WidgetRef) {}
        pub fn enable(_enabled: bool) {}
        pub fn detach(_target: &crate::core::widget::WidgetRef) {}
    }
}

pub use imp::ToolTip;
