//! Event dispatch and callback system for WinUI3 widgets.
//!
//! # Architecture
//!
//! Each top-level [`Frame`](super::app::Frame) owns a `Handlers` map
//! keyed by control id. Widgets register any number of command handlers
//! (`BN_CLICKED`, `EN_CHANGE`, `TBM_SETPOS`, …) on the frame; the
//! shared [`widget_wnd_proc`] trampoline walks the handler map to find
//! the first match for the incoming Win32 message and invokes it.
//!
//! A per-control (`HWND`-keyed) registry supplements the id-keyed
//! registry for widgets that were created with `HMENU = NULL` and
//! therefore don't have a stable id.
//!
//! # Subclassing
//!
//! Each widget installs a [`WNDPROC`](windows_sys) subclass via
//! [`SetWindowLongPtrW(hwnd, GWLP_WNDPROC, ...)`] that:
//! 1. Looks up the widget's `WidgetState` pointer stored in
//!    `GWLP_USERDATA`.
//! 2. Calls `process_message` with the per-window handler map.
//! 3. Forwards to the previous WndProc for messages we do not handle.

#![allow(non_snake_case)]

use std::cell::RefCell;
use std::collections::HashMap;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::Controls::NMHDR;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

type WindowKey = usize;
pub type CtrlId = u16;

/// Mouse button reported to handlers.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Payload carried by event handlers.
#[derive(Debug, Clone)]
pub enum WidgetEvent {
    /// `BN_CLICKED` from a button-style control.
    Clicked,
    /// Checkbox / Radio toggle with the new state.
    Toggled(bool),
    /// List/Choice/Combo selection change (carries new index, -1 if cleared).
    SelectionChanged(i32),
    /// Text-control content changed (`EN_CHANGE`).
    TextChanged(String),
    /// Slider / Gauge value changed (carries new value).
    ValueChanged(i32),
    /// Tab control switched page (carries new page index).
    PageChanged(usize),
    /// Generic mouse-down at `(x, y)` client coordinates.
    MouseDown(MouseButton, i32, i32),
    /// Generic mouse-up at `(x, y)` client coordinates.
    MouseUp(MouseButton, i32, i32),
    /// WM_CLOSE: the user requested that the window be closed.
    CloseRequested,
    /// The window is being destroyed (last chance to release resources).
    Destroy,
}

/// Signature of a widget event handler.
pub type Handler = Box<dyn FnMut(WidgetEvent)>;

/// Per-window handler table.
///
/// `commands` keys on control id (BN_CLICKED, EN_CHANGE, …) — the
/// mechanism used by 99 % of classic Win32 controls.
#[derive(Default)]
pub struct Handlers {
    /// Maps control id -> handler.
    pub commands: HashMap<CtrlId, Handler>,
    /// Mouse-down handlers keyed by (button, control id).
    pub mouse_down: HashMap<(MouseButton, CtrlId), Handler>,
    /// Mouse-up handlers keyed by (button, control id).
    pub mouse_up: HashMap<(MouseButton, CtrlId), Handler>,
    /// Optional handler for `WM_CLOSE`.
    pub close: Option<Handler>,
    /// Optional handler for `WM_NCDESTROY`.
    pub destroy: Option<Handler>,
}

thread_local! {
    static HANDLERS: RefCell<HashMap<WindowKey, Handlers>> = RefCell::new(HashMap::new());
    static HWND_HANDLERS: RefCell<HashMap<WindowKey, HwndHandlers>> = RefCell::new(HashMap::new());
}

/// Per-frame storage of HWND-keyed handlers.
#[derive(Default)]
pub struct HwndHandlers {
    pub map: HashMap<HWND, Vec<Handler>>,
}

/// Initialise the handler table for `hwnd` (replacing any prior one).
pub fn install_handlers(hwnd: HWND, handlers: Handlers) {
    HANDLERS.with(|m| m.borrow_mut().insert(hwnd as WindowKey, handlers));
}

/// Look up the handler table for `hwnd` and apply `f` to it.
pub fn with_handlers<R>(hwnd: HWND, f: impl FnOnce(&mut Handlers) -> R) -> Option<R> {
    HANDLERS.with(|m| m.borrow_mut().get_mut(&(hwnd as WindowKey)).map(f))
}

/// Install a single command handler for `(hwnd, id)`.
pub fn register_command<F: FnMut(WidgetEvent) + 'static>(hwnd: HWND, id: CtrlId, f: F) {
    HANDLERS.with(|m| {
        let mut m = m.borrow_mut();
        let entry = m.entry(hwnd as WindowKey).or_insert_with(Handlers::default);
        entry.commands.insert(id, Box::new(f));
    });
}

/// Install a single close handler for `hwnd`.
pub fn register_close<F: FnMut() + 'static>(hwnd: HWND, mut f: F) {
    HANDLERS.with(|m| {
        let mut m = m.borrow_mut();
        let entry = m.entry(hwnd as WindowKey).or_insert_with(Handlers::default);
        entry.close = Some(Box::new(move |_| f()));
    });
}

/// Install a handler keyed by child `HWND`.
pub fn register_hwnd<F: FnMut(WidgetEvent) + 'static>(parent_hwnd: HWND, child_hwnd: HWND, f: F) {
    HWND_HANDLERS.with(|m| {
        let mut m = m.borrow_mut();
        let entry = m
            .entry(parent_hwnd as WindowKey)
            .or_insert_with(HwndHandlers::default);
        entry
            .map
            .entry(child_hwnd)
            .or_insert_with(Vec::new)
            .push(Box::new(f));
    });
}

fn invoke_hwnd(parent_hwnd: HWND, child_hwnd: HWND, event: WidgetEvent) -> bool {
    let mut consumed = false;
    HWND_HANDLERS.with(|m| {
        let mut m = m.borrow_mut();
        if let Some(entry) = m.get_mut(&(parent_hwnd as WindowKey)) {
            if let Some(list) = entry.map.get_mut(&child_hwnd) {
                for handler in list.iter_mut() {
                    handler(event.clone());
                    consumed = true;
                }
            }
        }
    });
    consumed
}

fn remove_hwnd(parent_hwnd: HWND, child_hwnd: HWND) {
    HWND_HANDLERS.with(|m| {
        let mut m = m.borrow_mut();
        if let Some(entry) = m.get_mut(&(parent_hwnd as WindowKey)) {
            entry.map.remove(&child_hwnd);
        }
    });
}

/// Drop the table entries for `hwnd` on `WM_NCDESTROY`.
pub fn on_destroy(hwnd: HWND) {
    HANDLERS.with(|m| m.borrow_mut().remove(&(hwnd as WindowKey)));
    HWND_HANDLERS.with(|m| m.borrow_mut().remove(&(hwnd as WindowKey)));
}

// ============================================================================
// Convenience constructors.
// ============================================================================
impl Handlers {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `f` for command events emitted by the control with id `id`.
    pub fn on_command<F: FnMut(WidgetEvent) + 'static>(mut self, id: CtrlId, f: F) -> Self {
        self.commands.insert(id, Box::new(f));
        self
    }

    /// Register `f` for `WM_CLOSE` arriving on the frame.
    pub fn on_close<F: FnMut() + 'static>(mut self, mut f: F) -> Self {
        self.close = Some(Box::new(move |_| f()));
        self
    }

    /// Register `f` for `WM_NCDESTROY` arriving on the frame.
    pub fn on_destroy<F: FnMut() + 'static>(mut self, mut f: F) -> Self {
        self.destroy = Some(Box::new(move |_| f()));
        self
    }
}

// ============================================================================
// Dispatcher entry point.
// ============================================================================

/// Subclass trampoline entry. Installed via `SetWindowLongPtrW` on each
/// subclassed widget. Returns `true` if the message was consumed so
/// the caller can skip the default proc.
///
/// # Safety
///
/// This function is an `extern "system"` WndProc installed via
/// `SetWindowSubclass` (or `SetWindowLongPtrW`); it must be called
/// with the same `hwnd`/`msg`/`wparam`/`lparam` quartet Windows
/// delivers to a window procedure, and with the `hwnd` referring to
/// a live subclassed window.
pub unsafe extern "system" fn widget_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    let consumed = dispatch_frame(hwnd, msg, wparam, lparam);
    if consumed {
        return 0;
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

// ============================================================================
// dispatch_frame — top-level dispatch table.
// ============================================================================
fn dispatch_frame(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> bool {
    match msg {
        WM_COMMAND => dispatch_command(hwnd, wparam, lparam),
        WM_HSCROLL | WM_VSCROLL => dispatch_scroll(hwnd, msg, wparam, lparam),
        WM_LBUTTONDOWN | WM_MBUTTONDOWN | WM_RBUTTONDOWN => dispatch_mouse(hwnd, msg, lparam, true),
        WM_LBUTTONUP | WM_MBUTTONUP | WM_RBUTTONUP => dispatch_mouse(hwnd, msg, lparam, false),
        WM_CLOSE => dispatch_close(hwnd),
        WM_NCDESTROY => {
            on_destroy(hwnd);
            false
        }
        WM_NOTIFY => dispatch_notify(hwnd, lparam),
        WM_TIMER => dispatch_timer(hwnd, wparam),
        _ => false,
    }
}

// ============================================================================
// WM_COMMAND dispatcher.
// ============================================================================
fn dispatch_command(hwnd: HWND, wparam: usize, lparam: isize) -> bool {
    if hwnd.is_null() {
        return false;
    }
    let id = (wparam & 0xFFFF) as CtrlId;
    let code = ((wparam >> 16) & 0xFFFF) as u16;
    let target = hwnd;
    let child_hwnd = lparam as HWND;

    let event = match code {
        0 => Some(WidgetEvent::Clicked),
        1 => match current_class_name(target) {
            Some(name) if name.starts_with("COMBOBOX") => {
                Some(WidgetEvent::SelectionChanged(cb_get_cursel(target)))
            }
            Some(name) if name.starts_with("LISTBOX") => {
                Some(WidgetEvent::SelectionChanged(lb_get_cursel(target)))
            }
            _ => None,
        },
        0x300 => Some(WidgetEvent::TextChanged(current_text(target))),
        _ => None,
    };

    let event = match event {
        Some(e) => e,
        None => return false,
    };

    let mut consumed = false;
    if !child_hwnd.is_null() {
        consumed = invoke_hwnd(hwnd, child_hwnd, event.clone());
    }
    if !consumed && id != 0 {
        consumed = invoke_command(hwnd, id, event);
    }
    consumed
}

fn invoke_command(hwnd: HWND, id: CtrlId, event: WidgetEvent) -> bool {
    HANDLERS.with(|m| {
        let mut m = m.borrow_mut();
        if let Some(entry) = m.get_mut(&(hwnd as WindowKey)) {
            if let Some(handler) = entry.commands.get_mut(&id) {
                handler(event);
                return true;
            }
        }
        false
    })
}

// ============================================================================
// WM_NOTIFY dispatcher (tabs).
// ============================================================================
fn dispatch_notify(parent_hwnd: HWND, lparam: isize) -> bool {
    const TCN_SELCHANGE: u32 = 0xFFFFFDDA;
    if lparam == 0 {
        return false;
    }
    let hdr = lparam as *const NMHDR;
    if hdr.is_null() {
        return false;
    }
    let code = unsafe { (*hdr).code };
    if code != TCN_SELCHANGE {
        return false;
    }
    let tab_hwnd = unsafe { (*hdr).hwndFrom };
    if tab_hwnd.is_null() {
        return false;
    }
    let idx = unsafe { SendMessageW(tab_hwnd, 0x130B, 0, 0) } as usize;
    invoke_hwnd(parent_hwnd, tab_hwnd, WidgetEvent::PageChanged(idx))
}

// ============================================================================
// WM_TIMER dispatcher.
// ============================================================================
fn dispatch_timer(hwnd: HWND, wparam: usize) -> bool {
    crate::platform::winui3::timer::process_timer(hwnd, wparam)
}

// ============================================================================
// WM_SCROLL dispatcher.
// ============================================================================
fn dispatch_scroll(hwnd: HWND, _msg: u32, _wparam: usize, _lparam: isize) -> bool {
    let id = match unsafe { GetDlgCtrlID(hwnd) } as i32 {
        0 => return false,
        id => id as CtrlId,
    };
    invoke_command(hwnd, id, WidgetEvent::ValueChanged(current_value(hwnd)))
}

// ============================================================================
// WM_BUTTONDOWN/UP dispatcher.
// ============================================================================
fn dispatch_mouse(hwnd: HWND, msg: u32, lparam: isize, down: bool) -> bool {
    let button = match msg {
        WM_LBUTTONDOWN | WM_LBUTTONUP => MouseButton::Left,
        WM_MBUTTONDOWN | WM_MBUTTONUP => MouseButton::Middle,
        WM_RBUTTONDOWN | WM_RBUTTONUP => MouseButton::Right,
        _ => return false,
    };
    let id = unsafe { GetDlgCtrlID(hwnd) } as CtrlId;
    let x = (lparam & 0xFFFF) as i16 as i32;
    let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
    let event = if down {
        WidgetEvent::MouseDown(button, x, y)
    } else {
        WidgetEvent::MouseUp(button, x, y)
    };
    HANDLERS.with(|m| {
        let mut m = m.borrow_mut();
        if let Some(entry) = m.get_mut(&(hwnd as WindowKey)) {
            if let Some(handler) = entry.mouse_down.get_mut(&(button, id)) {
                handler(event);
                return true;
            }
        }
        false
    })
}

// ============================================================================
// WM_CLOSE / WM_NCDESTROY dispatchers.
// ============================================================================
fn dispatch_close(hwnd: HWND) -> bool {
    HANDLERS.with(|m| {
        let mut m = m.borrow_mut();
        if let Some(entry) = m.get_mut(&(hwnd as WindowKey)) {
            if let Some(handler) = entry.close.as_mut() {
                handler(WidgetEvent::CloseRequested);
                return true;
            }
        }
        false
    })
}

fn dispatch_destroy(hwnd: HWND) -> bool {
    HANDLERS.with(|m| {
        let mut m = m.borrow_mut();
        m.remove(&(hwnd as WindowKey));
    });
    false
}

// ============================================================================
// Internal helpers.
// ============================================================================
fn current_class_name(hwnd: HWND) -> Option<String> {
    unsafe {
        let mut buf = [0u16; 64];
        let len = GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        if len <= 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    }
}

fn current_text(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return String::new();
        }
        let mut buf: Vec<u16> = vec![0; len as usize + 1];
        GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1);
        String::from_utf16_lossy(&buf[..len as usize])
    }
}

fn current_value(hwnd: HWND) -> i32 {
    unsafe {
        match current_class_name(hwnd) {
            Some(name) if name.starts_with("msctls_trackbar32") => {
                SendMessageW(hwnd, 0x0400, 0, 0) as i32
            }
            Some(name) if name.starts_with("msctls_progress32") => {
                SendMessageW(hwnd, 0x0408, 0, 0) as i32
            }
            _ => 0,
        }
    }
}

fn cb_get_cursel(hwnd: HWND) -> i32 {
    unsafe { SendMessageW(hwnd, 0x0147, 0, 0) as i32 }
}

fn lb_get_cursel(hwnd: HWND) -> i32 {
    unsafe { SendMessageW(hwnd, 0x0188, 0, 0) as i32 }
}

// ============================================================================
// Convenience: fire a synthetic event into the per-HWND dispatcher.
// ============================================================================
pub fn dispatch(hwnd: HWND, _id: CtrlId, event: WidgetEvent) {
    HWND_HANDLERS.with(|m| {
        let mut m = m.borrow_mut();
        if let Some(entry) = m.get_mut(&(hwnd as WindowKey)) {
            for list in entry.map.values_mut() {
                for h in list.iter_mut() {
                    h(event.clone());
                }
            }
        }
    });
}

// ============================================================================
// Re-export the Win32 dialog IDs we may need elsewhere.
// ============================================================================
pub const CB_GETCURSEL: u32 = 0x0147;
pub const LB_GETCURSEL: u32 = 0x0188;
