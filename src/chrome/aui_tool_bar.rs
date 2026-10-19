//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! `wxAuiToolBar` — a dockable toolbar that can be detached to float as
//! a stand-alone window and re-docked to any of the four frame edges.
//!
//! Mirrors the user-visible behaviour of `wxAuiToolBar` from wxWidgets:
//! the toolbar is wrapped with a small "gripper" element at the leading
//! edge; clicking the gripper detaches the toolbar to a floating
//! top-level window, and the floating window can be re-docked by
//! clicking the gripper again, double-clicking the floating window's
//! title bar, or closing it.
//!
//! # Win32 implementation
//!
//! On Windows the toolbar itself is a standard `ToolbarWindow32` common
//! control (same as [`crate::ToolBar`]). The gripper is a small
//! `STATIC` child window drawn with a "≡" symbol that receives
//! `STN_CLICKED` notifications via `WM_COMMAND`. The floating window is
//! a `WS_POPUP | WS_CAPTION | WS_THICKFRAME | WS_SYSMENU` top-level
//! window backed by a custom-registered window class
//! (`RuWxAuiFloating`) whose WndProc re-docks on
//! `WM_NCLBUTTONDBLCLK` and on `WM_CLOSE`.
//!
//! On **macOS** and **Linux** the strip is a platform toolbar
//! hosted in the frame client area (`NSToolbar`-less legacy strip on
//! macOS, a `GtkBox` with flat buttons and a "≡" gripper on GTK4);
//! the floating window is a transient top-level popup. On any other
//! platform this is a no-op stub so the API is still available.

use std::cell::RefCell;
use std::rc::Rc;

// Backend delle strisce AUI: stesso sottoinsieme di funzioni su
// macOS (AppKit) e Linux (GTK4), quindi un alias condiviso.
#[cfg(target_os = "macos")]
use crate::platform::appkit as tb_backend;
#[cfg(target_os = "linux")]
use crate::platform::gtk as tb_backend;

use crate::core::geometry::Rect;
use crate::core::widget::Widget;
use crate::dc::image_list::ImageList;
use crate::window::frame::Frame;
use crate::window::panel::Panel;

use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use crate::platform::{as_hwnd, NativeHandle, NativeHandleExt};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;
// CallWindowProcW / SetWindowLongPtrW / SetPropW / GetPropW for the
// host-panel WndProc subclass.
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{CallWindowProcW, GetPropW, SetPropW};

// ── Win32 toolbar constants ─────────────────────────────────────
#[cfg(target_os = "windows")]
const TB_BUTTONSTRUCTSIZE: u32 = 0x041E;
#[cfg(target_os = "windows")]
const TB_SETBITMAPSIZE: u32 = 0x0420;
#[cfg(target_os = "windows")]
const TB_SETIMAGELIST: u32 = 0x0430;
#[cfg(target_os = "windows")]
const TB_ADDBUTTONS: u32 = 0x0444;
#[cfg(target_os = "windows")]
const TB_AUTOSIZE: u32 = 0x0421;
#[cfg(target_os = "windows")]
const TB_SETHOTIMAGELIST: u32 = 0x0434; // WM_USER + 52
#[cfg(target_os = "windows")]
const TB_DELETEBUTTON: u32 = 0x0416;
#[cfg(target_os = "windows")]
const TB_BUTTONCOUNT: u32 = 0x0418;
#[cfg(target_os = "windows")]
const CCM_SETVERSION: u32 = 0x2007; // CCM_FIRST (0x2000) + 0x7
#[cfg(target_os = "windows")]
const LPSTR_TEXTCALLBACK: isize = -1;
/// `TTN_GETDISPINFOA` — `TTN_FIRST - 0` (ANSI text request).
#[cfg(target_os = "windows")]
pub const TTN_GETDISPINFOA: i32 = -520;
/// `TTN_GETDISPINFOW` — `TTN_FIRST - 10` (wide text request).
///
/// A control sends whichever variant matches the format its parent
/// negotiated via `WM_NOTIFYFORMAT`. We answer **both** so the tooltip
/// shows the full label regardless of the negotiated format; filling
/// the wrong width makes it read garbage and show a single character.
#[cfg(target_os = "windows")]
pub const TTN_GETDISPINFOW: i32 = -530;
#[cfg(target_os = "windows")]
const TBSTYLE_FLAT: u32 = 0x0800;
/// `CCS_NORESIZE` — il toolbar non usa la larghezza/altezza del
/// contenitore come proprio dimensionamento di default.
const CCS_NORESIZE: u32 = 0x0040;
/// `CCS_NOPARENTALIGN` — impedisce al toolbar di ri-allinearsi
/// autonomamente al bordo del parent (default `CCS_TOP`): senza questo
/// stile ogni `MoveWindow` sul toolbar viene annullato dal controllo
/// che si riposiziona a `x=0` con larghezza = parent e altezza ideale
/// (overflow di 2px sopra il pannello host).
const CCS_NOPARENTALIGN: u32 = 0x8000;
#[cfg(target_os = "windows")]
const TBSTYLE_TOOLTIPS: u32 = 0x0100;
#[cfg(target_os = "windows")]
const TBSTYLE_BUTTON: u8 = 0x00;
#[cfg(target_os = "windows")]
const TBSTYLE_SEP: u8 = 0x01;
#[cfg(target_os = "windows")]
const TBSTATE_ENABLED: u8 = 0x04;

// STATIC control styles (not exported by `windows-sys` 0.59).
#[cfg(target_os = "windows")]
const SS_CENTER: u32 = 0x0000_0001;
#[cfg(target_os = "windows")]
const SS_NOTIFY: u32 = 0x0000_0100;

// Property names stored on the host panel via `SetPropW` so our
// subclassed WndProc can recover them. Using `SetPropW` is preferable
// to a global `HashMap<HWND, _>` because the properties are
// auto-removed when the host panel is destroyed.
#[cfg(target_os = "windows")]
const AUI_HOST_PROP_INNER: &str = "RuWxAuiHostInner";
#[cfg(target_os = "windows")]
const AUI_HOST_PROP_WNDPROC: &str = "RuWxAuiHostWndProc";
/// Proprietà sulla finestra `ToolbarWindow32` che conserva la sua WndProc
/// originale dopo la nostra sub-class (auto-rimossa alla distruzione).
#[cfg(target_os = "windows")]
const AUI_TB_PROP_WNDPROC: &str = "RuWxAuiToolbarWndProc";

#[cfg(target_os = "windows")]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(clippy::upper_case_acronyms)]
struct TBBUTTON {
    i_bitmap: i32,
    id_command: u32,
    fs_state: u8,
    fs_style: u8,
    _pad: u16,
    dw_data: usize,
    i_string: isize,
}

#[cfg(target_os = "windows")]
impl TBBUTTON {
    fn separator() -> Self {
        TBBUTTON {
            i_bitmap: 0,
            id_command: 0,
            fs_state: 0,
            fs_style: TBSTYLE_SEP,
            _pad: 0,
            dw_data: 0,
            i_string: 0,
        }
    }
}

const GRIPPER_WIDTH: i32 = 16;
const TOOLBAR_HEIGHT: i32 = crate::core::tokens::CONTROL_HEIGHT_COMPAT;
const FLOATING_CLASS: &str = "RuWxAuiFloating";

// ── Public API ──────────────────────────────────────────────────

/// Where the AuiToolBar is currently docked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuiDockSide {
    /// Docked along the top edge of the frame.
    Top,
    /// Docked along the bottom edge of the frame.
    Bottom,
    /// Docked along the left edge of the frame.
    Left,
    /// Docked along the right edge of the frame.
    Right,
    /// Floating as a stand-alone top-level window.
    Floating,
}

#[derive(Clone)]
#[allow(dead_code)]
enum ToolSpec {
    Separator,
    Tool {
        id: u16,
        image_index: i32,
        label: String,
    },
}

struct AuiToolBarInner {
    /// Wrapper for the host `RuWxPanel` that owns the gripper and the
    /// inner `ToolbarWindow32`. Keeping the Panel alive here ensures
    /// the host HWND (and therefore the gripper + inner toolbar that
    /// are its children) lives as long as the AuiToolBar does.
    #[cfg(target_os = "windows")]
    host_panel: Option<Panel>,
    /// Handle of the host `RuWxPanel` that wraps the gripper and the
    /// inner `ToolbarWindow32`. The AuiManager sizes the host panel;
    /// the subclassed WndProc re-lays out the children via `WM_SIZE`.
    #[cfg(target_os = "windows")]
    host_hwnd: NativeHandle,
    /// Handle of the inner `ToolbarWindow32` control.
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    /// Handle of the small gripper static control.
    #[cfg(target_os = "windows")]
    gripper_hwnd: HWND,
    /// Handle of the parent frame.
    #[cfg(target_os = "windows")]
    frame_hwnd: NativeHandle,
    /// Handle of the floating popup window (null when docked).
    #[cfg(target_os = "windows")]
    floating_hwnd: HWND,
    /// WM_COMMAND id reserved for the gripper. Dispatched by the
    /// frame's command handler to toggle float/dock.
    #[allow(dead_code)]
    gripper_id: u16,
    /// WM_COMMAND id of the toolbar child.
    #[allow(dead_code)]
    tb_id: u16,
    /// Current dock state.
    dock_side: AuiDockSide,
    /// Buffered button specs (separators + real buttons) that are
    /// sent to the control on `realize()`.
    buttons: Vec<ToolSpec>,
    /// NUL-terminated UTF-16 labels kept alive for the toolbar's
    /// lifetime. `TBBUTTON.iString` points into these buffers.
    toolbar_strings: Vec<Vec<u16>>,
    /// Lista icone conservata dalla toolbar: il clone condivide lo
    /// stesso `HIMAGELIST` (`Arc`), così l'handle inviato con
    /// `TB_SETIMAGELIST` resta valido anche se il chiamante fa cadere
    /// la propria `ImageList` a fine scope (prima l'handle diventava
    /// dangling e le icone sparivano).
    image_list: Option<ImageList>,
    visible: bool,
    rect: Rect,
    /// The bar height in pixels (top dock height / floating window
    /// height). Defaults to [`TOOLBAR_HEIGHT`] but can be changed at
    /// runtime via [`AuiToolBar::set_toolbar_height`] so the bar can
    /// host larger icons (e.g. 48–56 px) without truncation.
    toolbar_height: i32,
    /// Optional user callback fired when the dock state changes.
    on_dock_state_change: Option<Box<dyn FnMut(AuiDockSide)>>,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    stub_handle: crate::platform::NativeHandle,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    parent_handle: crate::platform::NativeHandle,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    frame_handle: crate::platform::NativeHandle,
}

#[derive(Clone)]
/// Dockable toolbar with a leading-edge gripper that detaches the
/// bar to a floating top-level window. Construct it with
/// [`AuiToolBar::new`], attach an image list, add tools, then call
/// [`AuiToolBar::realize`].
pub struct AuiToolBar {
    inner: Rc<RefCell<AuiToolBarInner>>,
}

// ── Win32 plumbing: floating window class & WndProc ─────────────

#[cfg(target_os = "windows")]
static FLOATING_CLASS_REGISTERED: std::sync::OnceLock<()> = std::sync::OnceLock::new();

#[cfg(target_os = "windows")]
fn ensure_floating_class_registered() {
    FLOATING_CLASS_REGISTERED.get_or_init(|| {
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let hinstance = GetModuleHandleW(std::ptr::null());
            let class_name = to_wide(FLOATING_CLASS);
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: 0,
                lpfnWndProc: Some(aui_floating_wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinstance,
                hIcon: std::ptr::null_mut(),
                hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
                hbrBackground: (COLOR_BTNFACE + 1) as usize as HBRUSH,
                lpszMenuName: std::ptr::null(),
                lpszClassName: class_name.as_ptr(),
                hIconSm: std::ptr::null_mut(),
            };
            RegisterClassExW(&wc);
        }
    });
}

/// WndProc for the floating popup window.
///
/// `GWLP_USERDATA` is set to a raw pointer to the active
/// `RefCell<AuiToolBarInner>` when the floating window is created. The
/// pointer is borrowed (no refcount management) — the AuiToolBar is
/// expected to be kept alive by user code while the toolbar is
/// floating.
#[cfg(target_os = "windows")]
unsafe extern "system" fn aui_floating_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCLBUTTONDBLCLK => {
            // Double-clicking the title bar re-docks to the top.
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let inner_ptr = ptr as *const RefCell<AuiToolBarInner>;
                // Borrow the inner, perform the dock, and let the
                // borrow drop before the WndProc returns.
                let inner_ref: &RefCell<AuiToolBarInner> = &*inner_ptr;
                let mut inner = inner_ref.borrow_mut();
                if !inner.floating_hwnd.is_null() {
                    do_dock(&mut inner, AuiDockSide::Top);
                }
            }
            0
        }
        WM_CLOSE => {
            // Treat the close button (X) as "re-dock to top" so the
            // toolbar never disappears — closing the floating window
            // simply puts the toolbar back in the frame.
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let inner_ref: &RefCell<AuiToolBarInner> =
                    &*(ptr as *const RefCell<AuiToolBarInner>);
                let mut inner = inner_ref.borrow_mut();
                if !inner.floating_hwnd.is_null() {
                    do_dock(&mut inner, AuiDockSide::Top);
                }
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_SIZE => {
            // The gripper + inner toolbar are children of the host
            // panel, which is a child of the floating window. We
            // just size the host panel; its subclassed WndProc
            // handles laying out the gripper + inner toolbar.
            let width = (lparam & 0xFFFF) as i32;
            let height = ((lparam >> 16) & 0xFFFF) as i32;
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let inner_ref: &RefCell<AuiToolBarInner> =
                    &*(ptr as *const RefCell<AuiToolBarInner>);
                let inner = inner_ref.borrow();
                if !inner.host_hwnd.is_null() {
                    MoveWindow(as_hwnd(inner.host_hwnd), 0, 0, width, height, 1);
                }
            }
            0
        }
        // `WM_COMMAND` reaches the floating window because the host
        // panel's subclass forwards it (gripper's `STN_CLICKED` is
        // `WM_COMMAND` with id = `gripper_id`). Route by id.
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as u16;
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 && id != 0 {
                let inner_ref: &RefCell<AuiToolBarInner> =
                    &*(ptr as *const RefCell<AuiToolBarInner>);
                let mut inner = inner_ref.borrow_mut();
                if id == inner.gripper_id && !inner.floating_hwnd.is_null() {
                    do_dock(&mut inner, AuiDockSide::Top);
                    return 0;
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_NOTIFY => {
            let nmhdr = lparam as *const NmHdr;
            let code = if nmhdr.is_null() { 0 } else { (*nmhdr).code };
            if code == TTN_GETDISPINFOW || code == TTN_GETDISPINFOA {
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
                if ptr != 0 {
                    let inner_ref: &RefCell<AuiToolBarInner> =
                        &*(ptr as *const RefCell<AuiToolBarInner>);
                    let inner = inner_ref.borrow();
                    fill_toolbar_ttn_dispinfo(lparam, &inner);
                    return 0;
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_DESTROY => {
            // Clear the GWLP_USERDATA pointer so a stray post-destroy
            // dispatch doesn't read a dangling reference.
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// ── Host panel WndProc ──────────────────────────────────────────
//
// The AuiToolBar's gripper and inner toolbar are re-parented to a
// dedicated host `RuWxPanel` child of the frame. The AuiManager calls
// `MoveWindow(host_hwnd, …)` to size the pane, so we subclass the
// panel's WndProc to handle `WM_SIZE` and re-layout the gripper and
// inner toolbar to fill the new client area. All other messages are
// forwarded to the original panel WndProc (preserving the panel's
// own bookkeeping in `GWLP_USERDATA`).
#[cfg(target_os = "windows")]
unsafe extern "system" fn aui_host_panel_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // Look up our subclass bookkeeping (auto-cleaned by Windows when
    // the host panel is destroyed). If it's gone, just defer to
    // DefWindowProcW.
    let inner_prop = to_wide(AUI_HOST_PROP_INNER);
    let orig_prop = to_wide(AUI_HOST_PROP_WNDPROC);
    let inner_handle = GetPropW(hwnd, inner_prop.as_ptr());
    let orig_handle = GetPropW(hwnd, orig_prop.as_ptr());

    if !inner_handle.is_null() && !orig_handle.is_null() && msg == WM_SIZE {
        let width = (lparam & 0xFFFF) as i32;
        let height = ((lparam >> 16) & 0xFFFF) as i32;
        // SAFETY: `inner_handle` was stored as a `*const
        // RefCell<AuiToolBarInner>` and the AuiToolBar is alive for
        // the lifetime of the host panel.
        let inner_ptr = inner_handle as *const RefCell<AuiToolBarInner>;
        let inner_ref: &RefCell<AuiToolBarInner> = &*inner_ptr;
        if let Ok(inner) = inner_ref.try_borrow_mut() {
            if !inner.gripper_hwnd.is_null() {
                MoveWindow(inner.gripper_hwnd, 0, 0, GRIPPER_WIDTH, height, 1);
            }
            if !inner.hwnd.is_null() {
                MoveWindow(
                    inner.hwnd,
                    GRIPPER_WIDTH,
                    0,
                    (width - GRIPPER_WIDTH).max(1),
                    height,
                    1,
                );
            }
        }
    }

    // Forward `WM_COMMAND` up the parent chain so the gripper's
    // `STN_CLICKED` (delivered as `WM_COMMAND` with id = gripper_id)
    // still reaches the frame, which owns the command handler that
    // toggles float/dock. Returning `DefWindowProcW` here would
    // consume the message at the host panel level and the frame
    // would never see it.
    if !inner_handle.is_null() && msg == WM_COMMAND {
        let parent = GetParent(hwnd);
        if !parent.is_null() {
            return SendMessageW(parent, WM_COMMAND, wparam, lparam);
        }
    }

    if !orig_handle.is_null() {
        let original: WNDPROC = std::mem::transmute(orig_handle);
        CallWindowProcW(original, hwnd, msg, wparam, lparam)
    } else {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

// ── Sub-class del `ToolbarWindow32` ────────────────────────────
// Il controllo `ToolbarWindow32` (comctl32) **ignora** i `MoveWindow`
// esterni: ri-imposta autonomamente la propria geometria a `x=0`,
// larghezza = client del parent e altezza ideale (comportamento
// `CCS_TOP`), nonostante `CCS_NOPARENTALIGN | CCS_NORESIZE` impostati
// alla creazione (verificato con test in isolamento: le finestre
// `STATIC` nello stesso parent si spostano regolarmente, il toolbar
// no). Per rendere efficace ogni `MoveWindow`/`TB_AUTOSIZE` esterno
// sub-classiamo il controllo intercettando `WM_WINDOWPOSCHANGING`
// (messaggio in/out): eseguiamo prima la WndProc originale e poi
// sovrascriviamo la geometria desiderata — subito dopo il gripper,
// a tutta l'altezza dell'host — così il sistema applica i nostri
// valori e l'overflow/alineamento restano stabili.
#[cfg(target_os = "windows")]
#[repr(C)]
struct AuiWindowPos {
    hwnd: HWND,
    hwnd_insert_after: HWND,
    x: i32,
    y: i32,
    cx: i32,
    cy: i32,
    flags: u32,
}

/// WndProc sub-class del `ToolbarWindow32`: vedi [`aui_toolbar_wnd_proc`].
/// # Sicurezza
/// `unsafe extern "system"` — FFI Win32; `lparam` di
/// `WM_WINDOWPOSCHANGING` è un `AuiWindowPos*` valido per tutta la
/// durata del messaggio.
#[cfg(target_os = "windows")]
unsafe extern "system" fn aui_toolbar_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let orig_prop = to_wide(AUI_TB_PROP_WNDPROC);
    let orig_handle = GetPropW(hwnd, orig_prop.as_ptr());

    if msg == WM_WINDOWPOSCHANGING && !orig_handle.is_null() {
        let original: WNDPROC = std::mem::transmute(orig_handle);
        // Lasciamo eseguire la logica interna del controllo…
        let ret = CallWindowProcW(original, hwnd, msg, wparam, lparam);
        // …poi imponiamo noi la geometria: il sistema usa i valori
        // finali della struttura in/out per applicare il movimento.
        let parent = GetParent(hwnd);
        if !parent.is_null() {
            let mut rc: RECT = std::mem::zeroed();
            if GetClientRect(parent, &mut rc) != 0 {
                let w = rc.right - rc.left;
                let h = rc.bottom - rc.top;
                if w > 0 && h > 0 {
                    // SAFETY: messaggio in/out con `AuiWindowPos*` valido.
                    let pos = &mut *(lparam as *mut AuiWindowPos);
                    pos.x = GRIPPER_WIDTH;
                    pos.y = 0;
                    pos.cx = (w - GRIPPER_WIDTH).max(1);
                    pos.cy = h;
                }
            }
        }
        return ret;
    }

    if !orig_handle.is_null() {
        let original: WNDPROC = std::mem::transmute(orig_handle);
        CallWindowProcW(original, hwnd, msg, wparam, lparam)
    } else {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

// ── Internal helpers (Win32) ────────────────────────────────────

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(dead_code)]
struct NmHdr {
    hwnd_from: HWND,
    id_from: usize,
    code: i32,
}

/// Local mirror of `NMTTDISPINFOW`. Only `hdr`, `lpsz_text` and
/// `sz_text` are read; the remaining fields exist purely to reproduce
/// the C struct layout so the pointer offsets line up.
#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(dead_code)]
struct NmTtDispInfoW {
    hdr: NmHdr,
    lpsz_text: *mut u16,
    sz_text: [u16; 80],
    hinst: HINSTANCE,
    u_flags: u32,
    l_param: isize,
}

#[cfg(target_os = "windows")]
fn toolbar_label_for_id(inner: &AuiToolBarInner, cmd_id: u32) -> Option<&str> {
    inner.buttons.iter().find_map(|spec| match spec {
        ToolSpec::Tool { id, label, .. } if *id as u32 == cmd_id => Some(label.as_str()),
        _ => None,
    })
}

#[cfg(target_os = "windows")]
unsafe fn fill_toolbar_ttn_dispinfo(lparam: LPARAM, inner: &AuiToolBarInner) {
    let info = lparam as *mut NmTtDispInfoW;
    if info.is_null() {
        return;
    }
    // For a toolbar tool registered without `TTF_IDISHWND`, the button's
    // command id is reported in `NMHDR.idFrom` — NOT in the `uFlags`
    // field. Reading the wrong field looks up a non-existent tool and
    // leaves the tooltip empty.
    let cmd_id = (*info).hdr.id_from as u32;
    let Some(label) = toolbar_label_for_id(inner, cmd_id) else {
        return;
    };
    // `szText` starts at the same offset in NMTTDISPINFO**A** and
    // NMTTDISPINFO**W**; only the element width differs. Pick the width
    // from the notification code so the control reads back the right
    // string instead of stopping at the first embedded NUL byte (which
    // is what produces the "single character" tooltip).
    let sz = std::ptr::addr_of_mut!((*info).sz_text);
    if (*info).hdr.code == TTN_GETDISPINFOA {
        // ANSI: write one byte per character.
        let bytes: Vec<u8> = label.chars().map(|c| c as u32 as u8).collect();
        let copy_len = bytes.len().min(79);
        let dst = sz.cast::<u8>();
        for (i, b) in bytes.iter().take(copy_len).enumerate() {
            *dst.add(i) = *b;
        }
        *dst.add(copy_len) = 0;
        (*info).lpsz_text = dst.cast::<u16>();
    } else {
        // Wide: write one UTF-16 code unit per character.
        let wide = to_wide(label);
        let copy_len = wide.len().saturating_sub(1).min(79);
        let dst = sz.cast::<u16>();
        for (i, w) in wide.iter().take(copy_len).enumerate() {
            *dst.add(i) = *w;
        }
        *dst.add(copy_len) = 0;
        (*info).lpsz_text = dst;
    }
}

#[cfg(target_os = "windows")]
fn do_float(inner_rc: &Rc<RefCell<AuiToolBarInner>>) {
    // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
    unsafe {
        // 1. Where is the cursor? Anchor the floating window near it.
        let mut pt: POINT = std::mem::zeroed();
        GetCursorPos(&mut pt);

        ensure_floating_class_registered();

        // 2. Compute a reasonable default size for the floating
        // window based on the frame's client width.
        let frame_hwnd = inner_rc.borrow().frame_hwnd;
        let mut frame_rect: RECT = std::mem::zeroed();
        GetClientRect(as_hwnd(frame_hwnd), &mut frame_rect);
        let width = (frame_rect.right - frame_rect.left).max(200);

        // 3. Create the floating window.
        let bar_height = inner_rc.borrow().toolbar_height;
        let class_name = to_wide(FLOATING_CLASS);
        let title = to_wide("Toolbar");
        let floating = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_POPUP | WS_CAPTION | WS_THICKFRAME | WS_SYSMENU | WS_VISIBLE,
            pt.x - 20,
            pt.y - 10,
            width,
            bar_height + 12,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            GetModuleHandleW(std::ptr::null()),
            std::ptr::null_mut(),
        );
        if floating.is_null() {
            return;
        }

        // 4. Re-parent the HOST PANEL to the floating window. The
        // gripper + inner toolbar remain children of the host panel,
        // so they come along for the ride. The host panel's
        // subclassed WndProc re-lays them out on `WM_SIZE` when we
        // size the host panel below.
        let host_hwnd = inner_rc.borrow().host_hwnd;

        // Hide the host panel before re-parenting to avoid a
        // one-frame flicker.
        ShowWindow(as_hwnd(host_hwnd), SW_HIDE);

        // SetParent returns the previous parent (the frame).
        SetParent(as_hwnd(host_hwnd), floating);

        // Size the host panel to fill the floating window's client
        // area. This triggers `WM_SIZE` on the host panel, which in
        // turn lays out the gripper + inner toolbar.
        MoveWindow(as_hwnd(host_hwnd), 0, 0, width, bar_height, 1);
        // Re-show the host panel inside the new parent.
        ShowWindow(as_hwnd(host_hwnd), SW_SHOW);
        // Make the toolbar recompute its internal layout (e riporta
        // gripper/toolbar alla geometria dell'host: `TB_AUTOSIZE` da
        // solo può rendere il toolbar più alto del contenitore).
        autosize_toolbar_clamped(&inner_rc.borrow());

        // 5. Stash the floating hwnd + inner pointer for the WndProc.
        {
            let mut inner = inner_rc.borrow_mut();
            inner.floating_hwnd = floating;
            inner.dock_side = AuiDockSide::Floating;
            // Borrowed raw pointer — the AuiToolBar keeps `inner_rc`
            // alive, so this is safe to read inside the WndProc.
            let inner_raw = inner_rc.as_ptr();
            SetWindowLongPtrW(floating, GWLP_USERDATA, inner_raw as isize);
        }

        // 6. Fire the dock-state-change callback, if any.
        fire_dock_state_change(inner_rc, AuiDockSide::Floating);
    }
}

#[cfg(target_os = "windows")]
fn do_dock(inner: &mut AuiToolBarInner, side: AuiDockSide) {
    // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
    unsafe {
        // If we were floating, destroy the floating window first.
        if !inner.floating_hwnd.is_null() {
            // Setting GWLP_USERDATA to 0 BEFORE DestroyWindow prevents
            // the floating WndProc from reading stale state during
            // the destroy.
            SetWindowLongPtrW(inner.floating_hwnd, GWLP_USERDATA, 0);
            DestroyWindow(inner.floating_hwnd);
            inner.floating_hwnd = std::ptr::null_mut();
        }

        // Re-parent the HOST PANEL (which contains the gripper and
        // inner toolbar as children) back to the frame. The gripper
        // and inner toolbar stay parented to the host panel — they
        // are its children — so we only need one SetParent call.
        SetParent(as_hwnd(inner.host_hwnd), as_hwnd(inner.frame_hwnd));

        // Position the host panel at the requested edge of the
        // frame's client area. We only honour the requested side
        // for layout; the toolbar itself is always rendered
        // horizontally. The host panel's subclassed WndProc will
        // re-lay out the gripper + inner toolbar to fill the new
        // client area in response to the implied `WM_SIZE`.
        let mut frame_rect: RECT = std::mem::zeroed();
        GetClientRect(as_hwnd(inner.frame_hwnd), &mut frame_rect);
        let fw = (frame_rect.right - frame_rect.left).max(100);
        let fh = (frame_rect.bottom - frame_rect.top).max(100);
        let bar_h = inner.toolbar_height;
        let (x, y, w, h) = match side {
            AuiDockSide::Top => (0, 0, fw, bar_h),
            AuiDockSide::Bottom => (0, fh - bar_h, fw, bar_h),
            AuiDockSide::Left => (0, 0, fw, bar_h),
            AuiDockSide::Right => (fw - 200, 0, 200, bar_h),
            AuiDockSide::Floating => unreachable!("do_dock called with Floating"),
        };
        MoveWindow(as_hwnd(inner.host_hwnd), x, y, w, h, 1);
        // Make the inner toolbar recompute its internal layout (poi
        // ri-allinea l'altezza all'host: vedi `autosize_toolbar_clamped`).
        autosize_toolbar_clamped(inner);

        // Make sure the host panel and its children are visible
        // after the re-parent.
        ShowWindow(as_hwnd(inner.host_hwnd), SW_SHOW);

        inner.dock_side = side;
    }
}

/// `TB_AUTOSIZE` + ri-allineamento alla geometria dell'host panel.
///
/// `TB_AUTOSIZE` ricalcola **entrambe** le dimensioni del `ToolbarWindow32`
/// in base al contenuto (icone + padding): con icone 40px il risultato è
/// 52px, 2px in più dell'host panel (50px), e l'eccedenza finiva sopra il
/// contenuto della finestra (notebook/pannelli). Qui l'auto-size viene
/// eseguito per il re-layout interno dei bottoni, poi gripper e toolbar
/// vengono ri-posizionati esattamente come fa il gestore `WM_SIZE` del
/// pannello host, eliminando l'overflow.
///
/// Se l'host non è ancora stato ridimensionato (client area vuota) la
/// geometria corretta arriva con il primo `WM_SIZE`, quindi non si fa nulla.
///
/// # Sicurezza
/// Chiamare solo da un contesto `unsafe`: esegue FFI Win32 (`SendMessageW`,
/// `GetClientRect`, `MoveWindow`). `MoveWindow` sui **figli** non genera
/// `WM_SIZE` sull'host, quindi non rientra nel `try_borrow_mut` della
/// subclass del pannello.
#[cfg(target_os = "windows")]
unsafe fn autosize_toolbar_clamped(inner: &AuiToolBarInner) {
    if inner.hwnd.is_null() || inner.host_hwnd.is_null() {
        return;
    }
    // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
    unsafe {
        SendMessageW(inner.hwnd, TB_AUTOSIZE, 0, 0);

        let mut rc: RECT = std::mem::zeroed();
        GetClientRect(as_hwnd(inner.host_hwnd), &mut rc);
        let w = rc.right - rc.left;
        let h = rc.bottom - rc.top;
        if w <= 0 || h <= 0 {
            // Host non ancora ridimensionato: niente da correggere.
            return;
        }
        if !inner.gripper_hwnd.is_null() {
            MoveWindow(inner.gripper_hwnd, 0, 0, GRIPPER_WIDTH, h, 1);
        }
        MoveWindow(
            inner.hwnd,
            GRIPPER_WIDTH,
            0,
            (w - GRIPPER_WIDTH).max(1),
            h,
            1,
        );
    }
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
fn fire_dock_state_change(inner_rc: &Rc<RefCell<AuiToolBarInner>>, side: AuiDockSide) {
    // Take the callback out so we don't hold a borrow during the
    // user's FnMut call.
    let cb = inner_rc.borrow_mut().on_dock_state_change.take();
    if let Some(mut cb) = cb {
        cb(side);
        inner_rc.borrow_mut().on_dock_state_change = Some(cb);
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn backend_dock_side(side: AuiDockSide) -> u8 {
    use tb_backend::{
        TOOLBAR_DOCK_BOTTOM, TOOLBAR_DOCK_LEFT, TOOLBAR_DOCK_RIGHT, TOOLBAR_DOCK_TOP,
    };
    match side {
        AuiDockSide::Top => TOOLBAR_DOCK_TOP,
        AuiDockSide::Bottom => TOOLBAR_DOCK_BOTTOM,
        AuiDockSide::Left => TOOLBAR_DOCK_LEFT,
        AuiDockSide::Right => TOOLBAR_DOCK_RIGHT,
        AuiDockSide::Floating => TOOLBAR_DOCK_TOP,
    }
}

/// Dock the backend strip to `side` and sync the cached dock side.
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn do_backend_dock(inner_rc: &Rc<RefCell<AuiToolBarInner>>, side: AuiDockSide) {
    let stub = inner_rc.borrow().stub_handle;
    tb_backend::toolbar_dock(stub, backend_dock_side(side));
    inner_rc.borrow_mut().dock_side = side;
}

/// Register the gripper click handler: alternates float/dock.
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn install_backend_gripper_handler(toolbar: &AuiToolBar, frame: &Frame) {
    let inner = toolbar.inner.clone();
    let gripper_id = toolbar.inner.borrow().gripper_id;
    frame.register_command_handler(
        gripper_id,
        Box::new(move || {
            let stub = inner.borrow().stub_handle;
            if tb_backend::toolbar_is_floating(stub) {
                do_backend_dock(&inner, AuiDockSide::Top);
                fire_dock_state_change(&inner, AuiDockSide::Top);
            } else {
                do_backend_float(&inner, 0, 0);
            }
        }),
    );
}

/// Detach the strip into a floating popup (no-op if already floating).
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn do_backend_float(inner_rc: &Rc<RefCell<AuiToolBarInner>>, x: i32, y: i32) {
    let stub = inner_rc.borrow().stub_handle;
    if tb_backend::toolbar_is_floating(stub) {
        return;
    }
    tb_backend::toolbar_toggle_float(stub, x, y);
    inner_rc.borrow_mut().dock_side = AuiDockSide::Floating;
    fire_dock_state_change(inner_rc, AuiDockSide::Floating);
}

// ── Public methods ──────────────────────────────────────────────

impl AuiToolBar {
    /// Create a new AuiToolBar docked to the top of `frame`. The
    /// gripper is shown at the leading edge of the toolbar; clicking
    /// it detaches the toolbar to a floating top-level window.
    pub fn new(frame: &Frame) -> Self {
        #[cfg(target_os = "windows")]
        {
            let frame_hwnd = frame.hwnd();
            let gripper_id = next_control_id();
            let tb_id = next_control_id();
            // Cache the current bar height locally so the initial
            // window creation honours whatever height has been
            // configured (e.g. via `set_toolbar_height` after
            // construction in user code that wraps `new`). For the
            // default path it is just `TOOLBAR_HEIGHT`.
            let bar_h = TOOLBAR_HEIGHT;

            // Create the host panel FIRST so we can re-parent the
            // gripper and the inner toolbar to it. The host panel
            // HWND is the one returned by `native_handle()` — that
            // is what the AuiManager calls `MoveWindow` on. The
            // subclassed WndProc (installed below) re-lays out the
            // children via `WM_SIZE`.
            let host_panel = Panel::new(frame);
            let host_hwnd = host_panel.hwnd();

            let inner = Rc::new(RefCell::new(AuiToolBarInner {
                host_panel: Some(host_panel),
                host_hwnd,
                hwnd: std::ptr::null_mut(),
                gripper_hwnd: std::ptr::null_mut(),
                frame_hwnd,
                floating_hwnd: std::ptr::null_mut(),
                gripper_id,
                tb_id,
                dock_side: AuiDockSide::Top,
                buttons: Vec::new(),
                toolbar_strings: Vec::new(),
                image_list: None,
                visible: true,
                rect: Rect::new(0, 0, 0, 0),
                toolbar_height: bar_h,
                on_dock_state_change: None,
            }));

            // Create the gripper (a small STATIC with SS_NOTIFY that
            // sends STN_CLICKED via WM_COMMAND when clicked). Its
            // parent is the host panel — STN_CLICKED will reach the
            // subclassed host WndProc, which forwards it to the
            // frame.
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let gripper_hwnd = unsafe {
                let wide_text = to_wide("\u{2261}"); // ≡
                CreateWindowExW(
                    0,
                    to_wide("STATIC").as_ptr(),
                    wide_text.as_ptr(),
                    WS_CHILD | WS_VISIBLE | SS_CENTER | SS_NOTIFY | WS_BORDER,
                    0,
                    0,
                    GRIPPER_WIDTH,
                    bar_h,
                    as_hwnd(host_hwnd),
                    gripper_id as usize as HMENU,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };

            // Create the inner toolbar (ToolbarWindow32) as a child
            // of the host panel too, so TB_AUTOSIZE doesn't fight
            // with the AuiManager-driven MoveWindow on the host.
            // `CCS_NOPARENTALIGN | CCS_NORESIZE` disattivano
            // l'auto-allineamento di default (`CCS_TOP`): senza di essi
            // il controllo annulla ogni `MoveWindow` riportandosi a
            // `x=0`, larghezza = parent e altezza ideale (2px di
            // overflow sopra l'host panel).
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            let tb_hwnd = unsafe {
                let wide_class = to_wide("ToolbarWindow32");
                CreateWindowExW(
                    0,
                    wide_class.as_ptr(),
                    std::ptr::null(),
                    WS_CHILD
                        | WS_VISIBLE
                        | TBSTYLE_FLAT
                        | TBSTYLE_TOOLTIPS
                        | CCS_NORESIZE
                        | CCS_NOPARENTALIGN,
                    GRIPPER_WIDTH,
                    0,
                    400,
                    bar_h,
                    as_hwnd(host_hwnd),
                    tb_id as usize as HMENU,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };

            {
                let mut inner_b = inner.borrow_mut();
                inner_b.hwnd = tb_hwnd;
                inner_b.gripper_hwnd = gripper_hwnd;
            }

            // Comctl32 v6 — full-colour imagelist icons and wide tooltips.
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                SendMessageW(tb_hwnd, CCM_SETVERSION, 6, 0);
            }

            // ── Sub-class del `ToolbarWindow32` ──────────────────
            // Senza sub-class il controllo annulla ogni `MoveWindow`
            // esterno (rimettendosi a x=0/larghezza parent): la
            // `aui_toolbar_wnd_proc` intercetta `WM_WINDOWPOSCHANGING`
            // e impone la geometria desiderata (dopo il gripper, tutta
            // l'altezza dell'host). La WndProc originale è conservata
            // in una proprietà `SetPropW` (auto-rimossa alla
            // distruzione del controllo).
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                let orig_proc = SetWindowLongPtrW(
                    tb_hwnd,
                    GWLP_WNDPROC,
                    aui_toolbar_wnd_proc as *const () as isize,
                );
                let orig_prop = to_wide(AUI_TB_PROP_WNDPROC);
                SetPropW(tb_hwnd, orig_prop.as_ptr(), orig_proc as _);
            }

            // Use a nice Unicode-capable font for the gripper glyph.
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                let hfont = CreateFontW(
                    16,
                    0,
                    0,
                    0,
                    400,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET as u32,
                    OUT_DEFAULT_PRECIS as u32,
                    CLIP_DEFAULT_PRECIS as u32,
                    DEFAULT_QUALITY as u32,
                    FF_SWISS as u32,
                    to_wide("Segoe UI Symbol").as_ptr(),
                );
                SendMessageW(gripper_hwnd, WM_SETFONT, hfont as usize, 1);
            }

            // Position the controls inside the host panel (the panel
            // itself will be sized by the AuiManager; the WM_SIZE
            // handler installed below also re-positions the children
            // whenever the panel is resized).
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                MoveWindow(gripper_hwnd, 0, 0, GRIPPER_WIDTH, bar_h, 1);
                MoveWindow(tb_hwnd, GRIPPER_WIDTH, 0, 1, bar_h, 1);
            }

            // ── Subclass the host panel WndProc ──────────────────
            // The Panel class stores its own `Rc<RefCell<PanelInner>>`
            // in `GWLP_USERDATA`; we don't want to clobber that, so we
            // use `SetPropW` for our subclass bookkeeping (inner ptr +
            // original WndProc). Windows auto-removes these properties
            // when the host panel is destroyed.
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                let inner_raw = Rc::as_ptr(&inner) as isize;
                let inner_prop = to_wide(AUI_HOST_PROP_INNER);
                SetPropW(as_hwnd(host_hwnd), inner_prop.as_ptr(), inner_raw as _);

                let orig_proc = SetWindowLongPtrW(
                    as_hwnd(host_hwnd),
                    GWLP_WNDPROC,
                    aui_host_panel_wnd_proc as *const () as isize,
                );
                let orig_prop = to_wide(AUI_HOST_PROP_WNDPROC);
                SetPropW(as_hwnd(host_hwnd), orig_prop.as_ptr(), orig_proc as _);
            }

            // Wire the gripper click: toggle between float and
            // re-dock-to-top. Using `register_command_handler` routes
            // STN_CLICKED (received by the frame as WM_COMMAND with
            // id = gripper_id) to our closure. The host panel
            // subclass forwards WM_COMMAND up to its parent, so the
            // frame still receives it.
            let inner_for_gripper = inner.clone();
            // Toolbar buttons request tooltip text via TTN_GETDISPINFO;
            // the frame answers it from the tool labels.
            let inner_for_ttn = inner.clone();
            frame.register_ttn_dispinfo_handler(
                tb_id,
                Box::new(move |lparam| {
                    let inner = inner_for_ttn.borrow();
                    // SAFETY: Win32 passes a live `NMTTDISPINFO` pointer.
                    unsafe { fill_toolbar_ttn_dispinfo(lparam, &inner) };
                }),
            );

            frame.register_command_handler(
                gripper_id,
                Box::new(move || {
                    let is_floating_now = {
                        let inner = inner_for_gripper.borrow();
                        !inner.floating_hwnd.is_null()
                    };
                    if is_floating_now {
                        let mut inner = inner_for_gripper.borrow_mut();
                        do_dock(&mut inner, AuiDockSide::Top);
                        fire_dock_state_change(&inner_for_gripper, AuiDockSide::Top);
                    } else {
                        do_float(&inner_for_gripper);
                    }
                }),
            );

            AuiToolBar { inner }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let gripper_id = next_control_id();
            let tb_id = next_control_id();
            #[cfg(any(target_os = "macos", target_os = "linux"))]
            {
                let parent_handle = crate::platform::window_handle(frame);
                let stub_handle = tb_backend::create_toolbar_strip(
                    parent_handle,
                    parent_handle,
                    true,
                    gripper_id,
                    TOOLBAR_HEIGHT,
                );
                let inner = Rc::new(RefCell::new(AuiToolBarInner {
                    gripper_id,
                    tb_id,
                    dock_side: AuiDockSide::Top,
                    buttons: Vec::new(),
                    toolbar_strings: Vec::new(),
                    image_list: None,
                    visible: true,
                    rect: Rect::new(0, 0, 0, 0),
                    toolbar_height: TOOLBAR_HEIGHT,
                    on_dock_state_change: None,
                    stub_handle,
                    parent_handle,
                    frame_handle: parent_handle,
                }));
                let bar = AuiToolBar { inner };
                install_backend_gripper_handler(&bar, frame);
                bar
            }
            #[cfg(all(
                not(target_os = "windows"),
                not(target_os = "macos"),
                not(target_os = "linux")
            ))]
            {
                let _ = frame;
                AuiToolBar {
                    inner: Rc::new(RefCell::new(AuiToolBarInner {
                        gripper_id: 0,
                        tb_id: 0,
                        dock_side: AuiDockSide::Top,
                        buttons: Vec::new(),
                        toolbar_strings: Vec::new(),
                        image_list: None,
                        visible: true,
                        rect: Rect::new(0, 0, 0, 0),
                        toolbar_height: TOOLBAR_HEIGHT,
                        on_dock_state_change: None,
                    })),
                }
            }
        }
    }

    /// Attach an image list. Must be called before [`AuiToolBar::realize`].
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    pub fn set_image_list(&self, _image_list: &ImageList) {}

    #[cfg(target_os = "macos")]
    pub fn set_image_list(&self, image_list: &ImageList) {
        crate::platform::appkit::toolbar_set_image_list(
            self.inner.borrow().stub_handle,
            image_list.handle(),
            image_list.width(),
            image_list.height(),
        );
        // Conserva la lista: il clone condivide lo stesso handle nativo.
        self.inner.borrow_mut().image_list = Some(image_list.clone());
    }

    /// Attach an image list. Must be called before [`AuiToolBar::realize`].
    #[cfg(target_os = "windows")]
    pub fn set_image_list(&self, image_list: &ImageList) {
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let inner = self.inner.borrow();
            SendMessageW(inner.hwnd, TB_SETIMAGELIST, 0, image_list.handle());
            SendMessageW(inner.hwnd, TB_SETHOTIMAGELIST, 0, image_list.handle());
            // Match bitmap size to image list's image size
            let w = image_list.width();
            let h = image_list.height();
            let lparam = ((w as u32) & 0xFFFF) | (((h as u32) & 0xFFFF) << 16);
            SendMessageW(inner.hwnd, TB_SETBITMAPSIZE, 0, lparam as isize);
        }
        // Conserva la lista per tutta la vita della toolbar: senza
        // questo l'handle diventa dangling appena il chiamante fa
        // cadere la `ImageList` locale e le icone spariscono.
        self.inner.borrow_mut().image_list = Some(image_list.clone());
    }

    /// Add a tool button. The `image_index` is the index into the
    /// previously-attached image list.
    pub fn add_tool(&self, id: u16, label: &str, image_index: i32) {
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::toolbar_add_tool(
                self.inner.borrow().stub_handle,
                id,
                image_index,
            );
        }
        #[cfg(target_os = "linux")]
        {
            // GTK4: l'`ImageList` è uno stub, il pulsante usa la label.
            tb_backend::toolbar_add_tool(self.inner.borrow().stub_handle, id, label, false);
        }
        let mut inner = self.inner.borrow_mut();
        inner.buttons.push(ToolSpec::Tool {
            id,
            image_index,
            label: label.to_string(),
        });
    }

    /// Add a vertical separator.
    pub fn add_separator(&self) {
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::toolbar_add_separator(self.inner.borrow().stub_handle);
        }
        #[cfg(target_os = "linux")]
        {
            tb_backend::toolbar_add_separator(self.inner.borrow().stub_handle);
        }
        self.inner.borrow_mut().buttons.push(ToolSpec::Separator);
    }

    /// Commit the buffered buttons to the control. Call this once
    /// after all tools / separators have been added and the image list
    /// has been attached.
    pub fn realize(&self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let inner = self.inner.borrow();
            let hwnd = inner.hwnd;
            let specs: Vec<ToolSpec> = inner.buttons.clone();
            SendMessageW(
                hwnd,
                TB_BUTTONSTRUCTSIZE,
                std::mem::size_of::<TBBUTTON>() as usize,
                0,
            );
            // Replace any buttons from a previous `realize()` call.
            let count = SendMessageW(hwnd, TB_BUTTONCOUNT, 0, 0) as i32;
            for i in (0..count).rev() {
                SendMessageW(hwnd, TB_DELETEBUTTON, i as usize, 0);
            }
            let mut btns: Vec<TBBUTTON> = Vec::with_capacity(specs.len());
            for spec in specs.iter() {
                match spec {
                    ToolSpec::Separator => btns.push(TBBUTTON::separator()),
                    ToolSpec::Tool {
                        id, image_index, ..
                    } => {
                        btns.push(TBBUTTON {
                            i_bitmap: *image_index,
                            id_command: *id as u32,
                            fs_state: TBSTATE_ENABLED,
                            fs_style: TBSTYLE_BUTTON,
                            _pad: 0,
                            dw_data: 0,
                            // Request tooltip text on demand via
                            // `TTN_GETDISPINFO` (answered by the frame /
                            // floating window).
                            i_string: LPSTR_TEXTCALLBACK,
                        });
                    }
                }
            }
            SendMessageW(hwnd, TB_ADDBUTTONS, btns.len(), btns.as_ptr() as isize);
            // Ri-allinea gripper/toolbar all'host dopo l'auto-size:
            // impedisce l'overflow di 2px sopra il contenuto sottostante.
            autosize_toolbar_clamped(&inner);
        }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let stub = self.inner.borrow().stub_handle;
            // AUI hosts the toolbar inside a pane; native NSToolbar belongs in the title bar.
            tb_backend::toolbar_set_prefer_native(stub, false);
            tb_backend::toolbar_realize(stub);
            tb_backend::toolbar_bring_to_front(stub);
        }
    }

    /// Commit the buffered buttons using the **native** toolbar.
    ///
    /// - On **macOS** this uses the real `NSToolbar` in the window title bar,
    ///   giving the user a fully native look and feel.
    /// - On **Windows** this is identical to [`AuiToolBar::realize`] (the
    ///   native `ToolbarWindow32` common control is always used).
    ///
    /// Call this after all tools / separators have been added and the image
    /// list has been attached. Do **not** add the toolbar as an AuiManager
    /// pane when using the native variant on macOS (the toolbar lives in the
    /// title-bar area, not in the client area).
    pub fn realize_native(&self) {
        #[cfg(target_os = "windows")]
        self.realize();
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let stub = self.inner.borrow().stub_handle;
            // prefer_native is already true by default — make sure it stays true.
            tb_backend::toolbar_set_prefer_native(stub, true);
            tb_backend::toolbar_realize(stub);
            tb_backend::toolbar_bring_to_front(stub);
        }
    }

    /// Change the height (in pixels) used for both the docked bar and
    /// the floating window. Call this **before** [`AuiToolBar::realize`]
    /// (or any time after, if you also re-position the bar via
    /// [`AuiToolBar::dock_to`]) so that icons larger than
    /// `TOOLBAR_HEIGHT` (28 px) can be hosted without truncation.
    ///
    /// A reasonable value when using 40×40 icons is `48`; for 48×48
    /// icons use `56`.
    /// Return the height in pixels reserved at the dock edge (or used
    /// by the floating window). Use this to add a spacer above the
    /// main client content so it is not covered by the docked bar.
    pub fn reserved_height(&self) -> i32 {
        #[cfg(target_os = "macos")]
        {
            if crate::platform::appkit::toolbar_is_native(self.inner.borrow().stub_handle) {
                return 0;
            }
        }
        self.inner.borrow().toolbar_height
    }

    /// Raise the host panel (and its children) above overlapping
    /// siblings (e.g. a notebook laid out by the frame sizer).
    pub fn bring_to_front(&self) {
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let inner = self.inner.borrow();
            if !inner.host_hwnd.is_null() {
                SetWindowPos(
                    as_hwnd(inner.host_hwnd),
                    HWND_TOP,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            tb_backend::toolbar_bring_to_front(self.inner.borrow().stub_handle);
        }
    }

    pub fn set_toolbar_height(&self, h: i32) {
        let h = h.max(TOOLBAR_HEIGHT);
        #[cfg(target_os = "windows")]
        {
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                // Snapshot the HWNDs and release the borrow *before* any
                // MoveWindow/SetWindowPos call, because those calls
                // synchronously deliver `WM_SIZE` to the host panel's
                // subclassed WndProc, which would otherwise fail with
                // `BorrowMutError` (the immutable borrow above would
                // still be alive while the WndProc tries to
                // `try_borrow_mut`).
                let (hwnd, host_hwnd, frame_hwnd, floating_hwnd) = {
                    let inner_b = self.inner.borrow();
                    (
                        inner_b.hwnd,
                        inner_b.host_hwnd,
                        inner_b.frame_hwnd,
                        inner_b.floating_hwnd,
                    )
                };
                if !hwnd.is_null() {
                    // L'host viene ri-posizionato subito dopo: qui
                    // serve solo il re-layout interno dei bottoni.
                    autosize_toolbar_clamped(&self.inner.borrow());
                }
                // Re-size the host panel so the gripper + inner
                // toolbar (children of the host) get the new height
                // via the host's subclassed `WM_SIZE` handler.
                if !host_hwnd.is_null() {
                    let mut frame_rect: RECT = std::mem::zeroed();
                    GetClientRect(as_hwnd(frame_hwnd), &mut frame_rect);
                    let fw = (frame_rect.right - frame_rect.left).max(200);
                    MoveWindow(as_hwnd(host_hwnd), 0, 0, fw, h, 1);
                }
                if !floating_hwnd.is_null() {
                    // Re-size the floating window so the host panel
                    // still fits. The floating WndProc's `WM_SIZE`
                    // handler will resize the host panel in turn.
                    SetWindowPos(
                        floating_hwnd,
                        std::ptr::null_mut(),
                        0,
                        0,
                        0,
                        h + 12,
                        SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
            }
        }
        self.inner.borrow_mut().toolbar_height = h;
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            tb_backend::toolbar_set_height(self.inner.borrow().stub_handle, h);
        }
        // Re-apply the current dock so any dock side uses the new
        // height for layout.
        #[cfg(target_os = "windows")]
        {
            let side = self.inner.borrow().dock_side;
            if side != AuiDockSide::Floating {
                let mut inner = self.inner.borrow_mut();
                do_dock(&mut inner, side);
            }
        }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let side = self.inner.borrow().dock_side;
            if side != AuiDockSide::Floating {
                do_backend_dock(&self.inner, side);
            }
        }
    }

    /// Re-apply native dock geometry (macOS). Call after [`Frame::set_sizer`]
    /// or other layout that may cover the toolbar strip.
    pub fn refresh_layout(&self) {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            tb_backend::toolbar_relayout_docked(self.inner.borrow().stub_handle);
        }
    }

    /// Programmatic dock/undock. `side` is one of
    /// [`AuiDockSide::Top`], [`AuiDockSide::Bottom`], [`AuiDockSide::Left`],
    /// [`AuiDockSide::Right`], or [`AuiDockSide::Floating`] (the
    /// last one is rejected — call [`AuiToolBar::float_at`] instead).
    pub fn dock_to(&self, side: AuiDockSide) {
        match side {
            AuiDockSide::Floating => {
                // dock_to(Floating) is a programming error — call
                // float_at(x, y) instead. We float to the cursor.
                self.float_at(0, 0);
            }
            side => {
                #[cfg(target_os = "windows")]
                {
                    let mut inner = self.inner.borrow_mut();
                    if inner.dock_side != side || !inner.floating_hwnd.is_null() {
                        do_dock(&mut inner, side);
                    }
                }
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                {
                    let inner_rc = self.inner.clone();
                    if inner_rc.borrow().dock_side != side
                        || tb_backend::toolbar_is_floating(inner_rc.borrow().stub_handle)
                    {
                        do_backend_dock(&inner_rc, side);
                        fire_dock_state_change(&inner_rc, side);
                    }
                }
                #[cfg(all(
                    not(target_os = "windows"),
                    not(target_os = "macos"),
                    not(target_os = "linux")
                ))]
                {
                    let _ = side;
                }
            }
        }
    }

    /// Where the toolbar is currently docked.
    pub fn dock_side(&self) -> AuiDockSide {
        self.inner.borrow().dock_side
    }

    /// `true` if the toolbar is currently floating as a stand-alone
    /// window.
    pub fn is_floating(&self) -> bool {
        #[cfg(target_os = "windows")]
        return !self.inner.borrow().floating_hwnd.is_null();
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        return tb_backend::toolbar_is_floating(self.inner.borrow().stub_handle);
        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        false
    }

    /// Detach the toolbar to a floating top-level window positioned
    /// at screen coordinates `(x, y)`. If `x == 0 && y == 0`, the
    /// floating window is anchored at the current cursor position.
    pub fn float_at(&self, x: i32, y: i32) {
        #[cfg(target_os = "windows")]
        {
            // We use do_float() to honour the proper "show at cursor"
            // logic; offset by (x, y) afterwards.
            do_float(&self.inner);
            if !(x == 0 && y == 0) {
                let inner = self.inner.borrow();
                if !inner.floating_hwnd.is_null() {
                    // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
                    unsafe {
                        SetWindowPos(
                            inner.floating_hwnd,
                            std::ptr::null_mut(),
                            x,
                            y,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                        );
                    }
                }
            }
        }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let inner_rc = self.inner.clone();
            if x == 0 && y == 0 {
                if tb_backend::toolbar_is_floating(inner_rc.borrow().stub_handle) {
                    do_backend_dock(&inner_rc, AuiDockSide::Top);
                    fire_dock_state_change(&inner_rc, AuiDockSide::Top);
                } else {
                    do_backend_float(&inner_rc, 0, 0);
                }
            } else {
                tb_backend::toolbar_float_at(inner_rc.borrow().stub_handle, x, y);
                if tb_backend::toolbar_is_floating(inner_rc.borrow().stub_handle) {
                    inner_rc.borrow_mut().dock_side = AuiDockSide::Floating;
                    fire_dock_state_change(&inner_rc, AuiDockSide::Floating);
                }
            }
        }
        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        {
            let _ = (x, y);
        }
    }

    /// Register a callback that fires whenever the dock state
    /// changes (toolbar was floated, re-docked, or moved to a
    /// different edge). The callback is `FnMut` and `'static` so it
    /// can own any state it needs.
    pub fn on_dock_state_change<F: FnMut(AuiDockSide) + 'static>(&self, callback: F) {
        self.inner.borrow_mut().on_dock_state_change = Some(Box::new(callback));
    }

    /// Register a callback that fires when any of the tools on this
    /// toolbar is clicked. The callback receives the id of the tool.
    pub fn on_tool_clicked<F: FnMut(u16) + 'static>(&self, frame: &Frame, callback: F) {
        #[cfg(target_os = "windows")]
        {
            // Share a single FnMut across all per-id handlers via
            // Rc<RefCell<...>>. We can't simply move `callback` into
            // the first closure because we need to clone it for
            // every registered tool id.
            let callback = std::rc::Rc::new(std::cell::RefCell::new(callback));

            let tool_ids: Vec<u16> = self
                .inner
                .borrow()
                .buttons
                .iter()
                .filter_map(|s| match s {
                    ToolSpec::Tool { id, .. } => Some(*id),
                    _ => None,
                })
                .collect();

            for id in tool_ids {
                let cb = callback.clone();
                frame.register_command_handler(id, Box::new(move || cb.borrow_mut()(id)));
            }
        }
        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        {
            let _ = (frame, callback);
        }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let callback = std::rc::Rc::new(std::cell::RefCell::new(callback));
            let tool_ids: Vec<u16> = self
                .inner
                .borrow()
                .buttons
                .iter()
                .filter_map(|s| match s {
                    ToolSpec::Tool { id, .. } => Some(*id),
                    _ => None,
                })
                .collect();
            for id in tool_ids {
                let cb = callback.clone();
                frame.register_command_handler(id, Box::new(move || cb.borrow_mut()(id)));
            }
        }
    }

    /// Return the native window handle of the inner toolbar.
    #[cfg(target_os = "windows")]
    pub fn hwnd(&self) -> HWND {
        self.inner.borrow().hwnd
    }

    /// Get a `WidgetRef` pointing at the inner toolbar state. Useful
    /// for APIs that need a generic widget handle, e.g. attaching a
    /// [`crate::ToolTip`] to the toolbar.
    pub fn as_widget_ref(&self) -> crate::core::widget::WidgetRef {
        self.inner.clone()
    }
}

// ── Widget trait ────────────────────────────────────────────────

impl Widget for AuiToolBarInner {
    fn native_handle(&self) -> isize {
        // Return the host panel HWND — the AuiManager sizes this
        // window, and the subclassed host WndProc re-lays out the
        // gripper + inner toolbar in response to `WM_SIZE`.
        #[cfg(target_os = "windows")]
        {
            self.host_hwnd
        }
        #[cfg(target_os = "macos")]
        {
            self.stub_handle
        }
        #[cfg(target_os = "linux")]
        {
            self.stub_handle
        }
        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        {
            0
        }
    }

    fn set_position(&mut self, x: i32, y: i32) {
        self.rect.x = x;
        self.rect.y = y;
        // (AuiToolBar manages its own layout; positioning the inner
        // toolbar doesn't move the gripper or the floating window.
        // Callers who want to control the toolbar's docked position
        // should use `dock_to`.)
        // Su Linux AuiManager è stub: la striscia si posiziona da sola.
        #[cfg(target_os = "linux")]
        {
            tb_backend::set_view_frame(self.parent_handle, self.stub_handle, self.rect);
        }
    }

    fn set_size(&mut self, w: u32, h: u32) {
        self.rect.width = w;
        self.rect.height = h;
        #[cfg(target_os = "linux")]
        {
            tb_backend::set_view_frame(self.parent_handle, self.stub_handle, self.rect);
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
            let show = if visible { SW_SHOW } else { SW_HIDE };
            // Show/hide the host panel; its children (gripper + inner
            // toolbar) come along for the ride.
            if !self.host_hwnd.is_null() {
                ShowWindow(as_hwnd(self.host_hwnd), show);
            }
            if !self.floating_hwnd.is_null() {
                ShowWindow(self.floating_hwnd, show);
            }
        }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            tb_backend::set_view_visible(self.stub_handle, visible);
        }
    }

    fn is_enabled(&self) -> bool {
        true
    }

    fn set_enabled(&mut self, _enabled: bool) {
        // AuiToolBar has no bar-level enabled state.
    }
}
