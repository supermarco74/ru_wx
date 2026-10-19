//! Win32 AUI pane hosts — caption bars, buttons, drag-to-float.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::OnceLock;

use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CreateSolidBrush, DeleteObject, DrawTextW, FillRect, GetStockObject, InvalidateRect, SetBkMode,
    SetTextColor, DT_LEFT, DT_SINGLELINE, DT_VCENTER, HBRUSH, TRANSPARENT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetCapture, ReleaseCapture, SetCapture};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetParent, GetWindowLongPtrW,
    MoveWindow, RegisterClassExW, SendMessageW, SetParent, SetWindowLongPtrW, ShowWindow,
    BS_PUSHBUTTON, BS_TEXT, CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA, HMENU, SW_SHOW, WM_COMMAND,
    WM_DESTROY, WM_HSCROLL, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_NOTIFY, WM_PAINT,
    WM_SIZE, WM_VSCROLL, WNDCLASSEXW, WS_CHILD, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_TABSTOP,
    WS_VISIBLE,
};

use crate::chrome::aui_manager::{
    AuiDockArt, AuiDockArtGradient, AuiManager, AuiPaneButton, AuiSashSpec,
};
use crate::chrome::aui_tool_bar::AuiDockSide;
use crate::core::geometry::Rect;
use crate::platform::ids::next_control_id;
use crate::platform::win32::to_wide;
use crate::platform::NativeHandle;
use crate::window::frame::Frame;

const PANE_HOST_CLASS: &str = "RuWxAuiPaneHost";
const PANE_BTN_WIDTH: i32 = 18;
const PANE_CAPTION_PAD: i32 = 6;
const DRAG_THRESHOLD: i32 = 5;

thread_local! {
    static PANE_HOSTS: RefCell<HashMap<(NativeHandle, String), Win32PaneHostState>> =
        RefCell::new(HashMap::new());
    static PANE_BTN_COMMANDS: RefCell<HashMap<u16, (NativeHandle, String, AuiPaneButton)>> =
        RefCell::new(HashMap::new());
    static PANE_FLOATS: RefCell<HashMap<(NativeHandle, String), HWND>> =
        RefCell::new(HashMap::new());
    static PANE_ACTION: RefCell<HashMap<NativeHandle, (*mut AuiManager, Frame)>> =
        RefCell::new(HashMap::new());
    static DRAG_STATE: RefCell<Option<DragState>> = const { RefCell::new(None) };
}

struct DragState {
    frame_handle: NativeHandle,
    pane_name: String,
    start_x: i32,
    start_y: i32,
    active: bool,
}

#[derive(Clone)]
struct Win32PaneHostState {
    host_handle: NativeHandle,
    host_hwnd: HWND,
    child_hwnd: HWND,
    frame_handle: NativeHandle,
    parent_handle: NativeHandle,
    pane_name: String,
    caption: String,
    dock_side: AuiDockSide,
    caption_height: u32,
    art: AuiDockArtSnapshot,
    floatable: bool,
    pinned: bool,
    buttons: Vec<(u16, HWND, AuiPaneButton)>,
}

#[derive(Clone)]
struct AuiDockArtSnapshot {
    active_caption: u32,
    inactive_caption: u32,
    /// Colore del filetto di separazione sotto la caption.
    border: u32,
    gradient: AuiDockArtGradient,
}

impl From<&AuiDockArt> for AuiDockArtSnapshot {
    fn from(art: &AuiDockArt) -> Self {
        Self {
            active_caption: art.active_caption(),
            inactive_caption: art.inactive_caption(),
            border: art.border(),
            gradient: art.gradient(),
        }
    }
}

fn ensure_host_class() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        // SAFETY: Win32 class registration.
        unsafe {
            // Bind the class-name buffer to a local so the `*const u16`
            // stays valid for the entire `RegisterClassExW` call (the
            // previous expression-level temporary was dropped before the
            // FFI call returned, producing a dangling pointer).
            let class_name = to_wide(PANE_HOST_CLASS);
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(pane_host_wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: GetModuleHandleW(std::ptr::null()),
                hIcon: std::ptr::null_mut(),
                hCursor: std::ptr::null_mut(),
                hbrBackground: GetStockObject(0) as HBRUSH,
                lpszMenuName: std::ptr::null(),
                lpszClassName: class_name.as_ptr(),
                hIconSm: std::ptr::null_mut(),
            };
            RegisterClassExW(&wc);
        }
    });
}

fn button_glyph(button: AuiPaneButton, pinned: bool) -> &'static str {
    match button {
        AuiPaneButton::Close => "x",
        AuiPaneButton::Maximize => "[]",
        AuiPaneButton::Minimize => "-",
        AuiPaneButton::Pin => {
            if pinned {
                "P"
            } else {
                "p"
            }
        }
        AuiPaneButton::Custom(_) => "*",
    }
}

fn darken(color: u32, amount: u32) -> u32 {
    let r = ((color >> 16) & 0xFF).saturating_sub(amount);
    let g = ((color >> 8) & 0xFF).saturating_sub(amount);
    let b = (color & 0xFF).saturating_sub(amount);
    (r << 16) | (g << 8) | b
}

fn colorref(c: u32) -> u32 {
    let r = (c >> 16) & 0xFF;
    let g = (c >> 8) & 0xFF;
    let b = c & 0xFF;
    r | (g << 8) | (b << 16)
}

/// Colore del testo (COLORREF) con contrasto garantito rispetto al
/// background della caption: nero su fondi chiari, bianco su fondi
/// scuri. Sostituisce il `SetTextColor(hdc, 0)` fisso, che rendeva
/// illeggibile il titolo con le caption scure (tema dark / preset
/// scuri dell'`AuiDockArt`).
fn contrast_text_colorref(bg: u32) -> u32 {
    let r = ((bg >> 16) & 0xFF) as f32;
    let g = ((bg >> 8) & 0xFF) as f32;
    let b = (bg & 0xFF) as f32;
    // Luminanza relativa BT.601 (la stessa usata percepita per RGB).
    let luminance = 0.299 * r + 0.587 * g + 0.114 * b;
    if luminance > 140.0 {
        0x0000_0000 // nero (COLORREF == RGB per il nero)
    } else {
        0x00FF_FFFF // bianco
    }
}

unsafe fn paint_caption(
    hdc: *mut std::ffi::c_void,
    width: i32,
    cap_h: i32,
    art: &AuiDockArtSnapshot,
    title: &str,
) {
    let top = art.active_caption;
    let bottom = darken(top, 24);
    match art.gradient {
        AuiDockArtGradient::None => {
            let brush = CreateSolidBrush(colorref(top));
            let rect = windows_sys::Win32::Foundation::RECT {
                left: 0,
                top: 0,
                right: width,
                bottom: cap_h,
            };
            FillRect(hdc, &rect, brush);
            DeleteObject(brush as _);
        }
        AuiDockArtGradient::Vertical | AuiDockArtGradient::Horizontal => {
            let steps = 20;
            for i in 0..steps {
                let t = i as f32 / (steps - 1).max(1) as f32;
                let r = (((top >> 16) & 0xFF) as f32 * (1.0 - t)
                    + (((bottom >> 16) & 0xFF) as f32 * t)) as u32;
                let g = (((top >> 8) & 0xFF) as f32 * (1.0 - t)
                    + (((bottom >> 8) & 0xFF) as f32 * t)) as u32;
                let b = ((top & 0xFF) as f32 * (1.0 - t) + ((bottom & 0xFF) as f32 * t)) as u32;
                let c = (r << 16) | (g << 8) | b;
                let brush = CreateSolidBrush(colorref(c));
                if matches!(art.gradient, AuiDockArtGradient::Vertical) {
                    let y0 = cap_h * i / steps;
                    let y1 = cap_h * (i + 1) / steps + 1;
                    let rect = windows_sys::Win32::Foundation::RECT {
                        left: 0,
                        top: y0,
                        right: width,
                        bottom: y1,
                    };
                    FillRect(hdc, &rect, brush);
                } else {
                    let x0 = width * i / steps;
                    let x1 = width * (i + 1) / steps + 1;
                    let rect = windows_sys::Win32::Foundation::RECT {
                        left: x0,
                        top: 0,
                        right: x1,
                        bottom: cap_h,
                    };
                    FillRect(hdc, &rect, brush);
                }
                DeleteObject(brush as _);
            }
        }
    }
    SetBkMode(hdc, TRANSPARENT as i32);
    // Colore del testo in base alla luminanza del fondo della caption
    // (media tra inizio e fine del gradiente): nero su fondi chiari,
    // bianco su fondi scuri.
    let avg = |sh: u32| -> u32 { (((top >> sh) & 0xFF) + ((bottom >> sh) & 0xFF)) / 2 };
    let mid = (avg(16) << 16) | (avg(8) << 8) | avg(0);
    SetTextColor(hdc, contrast_text_colorref(mid));
    let wide = to_wide(title);
    let mut rect = windows_sys::Win32::Foundation::RECT {
        left: PANE_CAPTION_PAD,
        top: 0,
        right: width - PANE_CAPTION_PAD,
        bottom: cap_h,
    };
    DrawTextW(
        hdc,
        wide.as_ptr(),
        -1,
        &mut rect,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE,
    );
    // Filetto di separazione sotto la caption con il colore
    // `AuiDockArt::border` (prima questo colore non veniva mai usato).
    if cap_h > 1 {
        let brush = CreateSolidBrush(colorref(art.border));
        let line = windows_sys::Win32::Foundation::RECT {
            left: 0,
            top: cap_h - 1,
            right: width,
            bottom: cap_h,
        };
        FillRect(hdc, &line, brush);
        DeleteObject(brush as _);
    }
}

unsafe fn layout_host(hwnd: HWND, cap_h: i32) {
    let mut client = windows_sys::Win32::Foundation::RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    GetClientRect(hwnd, &mut client);
    let w = client.right - client.left;
    let h = client.bottom - client.top;
    let child = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as HWND;
    if !child.is_null() {
        MoveWindow(child, 0, cap_h, w, (h - cap_h).max(0), 1);
    }
}

unsafe extern "system" fn pane_host_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_SIZE => {
            let cap_h = PANE_HOSTS.with(|m| {
                m.borrow()
                    .values()
                    .find(|s| s.host_hwnd == hwnd)
                    .map(|s| s.caption_height as i32)
                    .unwrap_or(21)
            });
            layout_host(hwnd, cap_h);
            0
        }
        WM_PAINT => {
            let state =
                PANE_HOSTS.with(|m| m.borrow().values().find(|s| s.host_hwnd == hwnd).cloned());
            if let Some(state) = state {
                let mut ps = std::mem::MaybeUninit::uninit();
                let hdc = windows_sys::Win32::Graphics::Gdi::BeginPaint(hwnd, ps.as_mut_ptr());
                if !hdc.is_null() {
                    let mut client = windows_sys::Win32::Foundation::RECT {
                        left: 0,
                        top: 0,
                        right: 0,
                        bottom: 0,
                    };
                    GetClientRect(hwnd, &mut client);
                    paint_caption(
                        hdc,
                        client.right,
                        state.caption_height as i32,
                        &state.art,
                        &state.caption,
                    );
                    windows_sys::Win32::Graphics::Gdi::EndPaint(hwnd, ps.as_ptr());
                }
            }
            0
        }
        WM_LBUTTONDOWN => {
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            if let Some(state) =
                PANE_HOSTS.with(|m| m.borrow().values().find(|s| s.host_hwnd == hwnd).cloned())
            {
                if y < state.caption_height as i32
                    && state.floatable
                    && !state.pinned
                    && !on_caption_button(&state, x, y)
                {
                    SetCapture(hwnd);
                    DRAG_STATE.with(|d| {
                        *d.borrow_mut() = Some(DragState {
                            frame_handle: state.frame_handle,
                            pane_name: state.pane_name.clone(),
                            start_x: x,
                            start_y: y,
                            active: false,
                        });
                    });
                }
            }
            0
        }
        WM_MOUSEMOVE => {
            DRAG_STATE.with(|d| {
                let mut slot = d.borrow_mut();
                if let Some(ref mut drag) = *slot {
                    if GetCapture() == hwnd {
                        let x = (lparam & 0xFFFF) as i16 as i32;
                        let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
                        let dx = (x - drag.start_x).abs();
                        let dy = (y - drag.start_y).abs();
                        if dx >= DRAG_THRESHOLD || dy >= DRAG_THRESHOLD {
                            drag.active = true;
                        }
                    }
                }
            });
            0
        }
        WM_LBUTTONUP => {
            let should_float = DRAG_STATE.with(|d| {
                let mut slot = d.borrow_mut();
                if let Some(drag) = slot.take() {
                    if drag.active {
                        return Some((drag.frame_handle, drag.pane_name));
                    }
                }
                None
            });
            if GetCapture() == hwnd {
                ReleaseCapture();
            }
            if let Some((frame_handle, pane_name)) = should_float {
                request_pane_float(frame_handle, &pane_name);
            }
            0
        }
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as u16;
            if let Some((frame_handle, pane_name, button)) =
                PANE_BTN_COMMANDS.with(|m| m.borrow().get(&id).cloned())
            {
                invoke_pane_button(frame_handle, &pane_name, button);
            } else {
                // I controlli dell'utente vivono dentro il pane: il
                // `RuWxPanel` li inoltra qui, ma questo wndproc prima
                // li assorbiva (return 0) e nessun `on_click`
                // registrato sul frame scattava mai. Ora l'id non
                // appartenente ai bottoni di caption risale al frame.
                let parent = GetParent(hwnd);
                if !parent.is_null() {
                    SendMessageW(parent, WM_COMMAND, wparam, lparam);
                }
            }
            0
        }
        WM_NOTIFY | WM_HSCROLL | WM_VSCROLL => {
            // Stesso principio del `WM_COMMAND`: le notifiche dei
            // controlli figli (tab control, slider, list…) devono
            // raggiungere il dispatcher del frame, non essere assorbite
            // da questo wndproc (`DefWindowProcW` le droppava).
            let parent = GetParent(hwnd);
            if !parent.is_null() {
                SendMessageW(parent, msg, wparam, lparam)
            } else {
                0
            }
        }
        WM_DESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Hit-test dei bottoni del caption. I rettangoli rispecchiano quelli
/// usati da [`position_aui_pane_host`]: banda di destra larga
/// `PANE_BTN_WIDTH` per ciascun bottone, `y` compreso in
/// `1 .. caption_height - 1`. Prima la funzione restituiva sempre
/// `false`, quindi un clic nella zona dei bottoni avviava comunque il
/// drag-to-float del pannello.
fn on_caption_button(state: &Win32PaneHostState, x: i32, y: i32) -> bool {
    if state.buttons.is_empty() {
        return false;
    }
    let cap_h = state.caption_height as i32;
    if y < 1 || y >= cap_h - 1 {
        return false;
    }
    let mut client = windows_sys::Win32::Foundation::RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: GetClientRect su un HWND valido posseduto dal thread.
    unsafe {
        GetClientRect(state.host_hwnd, &mut client);
    }
    let w = client.right - client.left;
    let first_x = w - state.buttons.len() as i32 * PANE_BTN_WIDTH;
    x >= first_x && x < w
}

fn invoke_pane_button(frame_handle: NativeHandle, pane_name: &str, button: AuiPaneButton) {
    PANE_ACTION.with(|m| {
        if let Some((mgr, frame)) = m.borrow().get(&frame_handle).cloned() {
            // SAFETY: refreshed each `AuiManager::update`.
            unsafe {
                if let Some(mgr) = mgr.as_mut() {
                    mgr.handle_pane_button(&frame, pane_name, button);
                }
            }
        }
    });
}

pub fn request_pane_float(frame_handle: NativeHandle, pane_name: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut pt = POINT { x: 0, y: 0 };
    // SAFETY: GetCursorPos out-param.
    unsafe {
        GetCursorPos(&mut pt);
    }
    PANE_ACTION.with(|m| {
        if let Some((mgr, frame)) = m.borrow().get(&frame_handle).cloned() {
            unsafe {
                if let Some(mgr) = mgr.as_mut() {
                    mgr.float_pane(&frame, pane_name, pt.x, pt.y);
                }
            }
        }
    });
}

pub fn register_aui_pane_action_target(
    frame_handle: NativeHandle,
    mgr: *mut AuiManager,
    frame: Frame,
) {
    PANE_ACTION.with(|m| {
        m.borrow_mut().insert(frame_handle, (mgr, frame));
    });
}

unsafe fn create_host_hwnd(parent: HWND, host_id: u16) -> HWND {
    ensure_host_class();
    CreateWindowExW(
        0,
        to_wide(PANE_HOST_CLASS).as_ptr(),
        to_wide("").as_ptr(),
        WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN,
        0,
        0,
        100,
        100,
        parent,
        host_id as usize as HMENU,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    )
}

/// Ricrea i bottoni del caption di un pannello AUI: distrugge
/// quelli esistenti e ne crea uno per ogni voce in `buttons`.
fn rebuild_buttons_from_list(
    state: &mut Win32PaneHostState,
    buttons: &[AuiPaneButton],
    pinned: bool,
) {
    unsafe {
        for (_, btn, _) in state.buttons.drain(..) {
            DestroyWindow(btn);
        }
    }
    PANE_BTN_COMMANDS.with(|map| {
        map.borrow_mut()
            .retain(|_, (fh, name, _)| !(*fh == state.frame_handle && name == &state.pane_name));
    });
    let cap_h = state.caption_height as i32;
    let mut idx = 0i32;
    for &button in buttons {
        let cmd = next_control_id();
        PANE_BTN_COMMANDS.with(|map| {
            map.borrow_mut()
                .insert(cmd, (state.frame_handle, state.pane_name.clone(), button));
        });
        let label = to_wide(button_glyph(button, pinned));
        // SAFETY: CreateWindowExW for caption button.
        let btn = unsafe {
            CreateWindowExW(
                0,
                to_wide("BUTTON").as_ptr(),
                label.as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_PUSHBUTTON as u32) | (BS_TEXT as u32),
                0,
                0,
                PANE_BTN_WIDTH,
                cap_h - 2,
                state.host_hwnd,
                cmd as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        state.buttons.push((cmd, btn, button));
        idx += 1;
    }
    let _ = idx;
}

#[allow(clippy::too_many_arguments)] // mirrors the per-pane Win32 host state; kept flat for FFI parity
pub fn ensure_aui_pane_host(
    frame_handle: NativeHandle,
    parent_handle: NativeHandle,
    pane_name: &str,
    child_handle: NativeHandle,
    dock_side: AuiDockSide,
    caption: &str,
    buttons: &[AuiPaneButton],
    art: &AuiDockArt,
    floatable: bool,
    pinned: bool,
) -> NativeHandle {
    let key = (frame_handle, pane_name.to_string());
    let parent = parent_handle as HWND;
    let child = child_handle as HWND;

    if let Some(mut state) = PANE_HOSTS.with(|m| m.borrow().get(&key).cloned()) {
        state.caption = caption.to_string();
        state.dock_side = dock_side;
        state.art = AuiDockArtSnapshot::from(art);
        state.caption_height = art.caption_height();
        state.floatable = floatable;
        state.pinned = pinned;
        if state.buttons.len() != buttons.len() {
            rebuild_buttons_from_list(&mut state, buttons, pinned);
        } else {
            for ((_, btn, kind), &new_kind) in state.buttons.iter().zip(buttons.iter()) {
                if *kind == AuiPaneButton::Pin {
                    let label = to_wide(button_glyph(AuiPaneButton::Pin, pinned));
                    // SAFETY: SetWindowTextW on owned button.
                    unsafe {
                        windows_sys::Win32::UI::WindowsAndMessaging::SetWindowTextW(
                            *btn,
                            label.as_ptr(),
                        );
                    }
                }
                let _ = new_kind;
            }
        }
        PANE_HOSTS.with(|m| m.borrow_mut().insert(key.clone(), state));
        return PANE_HOSTS.with(|m| m.borrow()[&key].host_handle);
    }

    let host_id = next_control_id();
    // SAFETY: host window creation.
    let host_hwnd = unsafe { create_host_hwnd(parent, host_id) };
    // SAFETY: reparent pane child into host.
    unsafe {
        SetParent(child, host_hwnd);
        SetWindowLongPtrW(host_hwnd, GWLP_USERDATA, child as isize);
    }

    let mut state = Win32PaneHostState {
        host_handle: host_hwnd as NativeHandle,
        host_hwnd,
        child_hwnd: child,
        frame_handle,
        parent_handle,
        pane_name: pane_name.to_string(),
        caption: caption.to_string(),
        dock_side,
        caption_height: art.caption_height(),
        art: AuiDockArtSnapshot::from(art),
        floatable,
        pinned,
        buttons: Vec::new(),
    };
    rebuild_buttons_from_list(&mut state, buttons, pinned);
    PANE_HOSTS.with(|m| m.borrow_mut().insert(key, state));
    host_hwnd as NativeHandle
}

pub fn position_aui_pane_host(
    parent_handle: NativeHandle,
    host_handle: NativeHandle,
    rect: Rect,
    caption_height: u32,
) {
    let host = host_handle as HWND;
    let parent = parent_handle as HWND;
    // SAFETY: MoveWindow / SetParent on owned HWNDs.
    unsafe {
        if windows_sys::Win32::UI::WindowsAndMessaging::GetParent(host) != parent {
            SetParent(host, parent);
        }
        MoveWindow(
            host,
            rect.x,
            rect.y,
            rect.width as i32,
            rect.height as i32,
            1,
        );
        ShowWindow(host, SW_SHOW);
    }
    PANE_HOSTS.with(|m| {
        if let Some(state) = m
            .borrow_mut()
            .values_mut()
            .find(|s| s.host_handle == host_handle)
        {
            state.caption_height = caption_height;
            let cap_h = caption_height as i32;
            let w = rect.width as i32;
            let mut x = w - state.buttons.len() as i32 * PANE_BTN_WIDTH;
            for (_, btn, _) in &state.buttons {
                unsafe {
                    MoveWindow(*btn, x, 1, PANE_BTN_WIDTH, cap_h - 2, 1);
                }
                x += PANE_BTN_WIDTH;
            }
            unsafe {
                layout_host(host, cap_h);
                InvalidateRect(host, std::ptr::null(), 1);
            }
        }
    });
}

pub fn aui_pane_host_handle(frame_handle: NativeHandle, pane_name: &str) -> Option<NativeHandle> {
    PANE_HOSTS.with(|m| {
        m.borrow()
            .get(&(frame_handle, pane_name.to_string()))
            .map(|s| s.host_handle)
    })
}

pub fn pane_button_commands_for_frame(
    frame_handle: NativeHandle,
) -> Vec<(u16, String, AuiPaneButton)> {
    PANE_BTN_COMMANDS.with(|map| {
        map.borrow()
            .iter()
            .filter(|(_, (fh, _, _))| *fh == frame_handle)
            .map(|(id, (_, name, btn))| (*id, name.clone(), *btn))
            .collect()
    })
}

pub fn sync_aui_pane_host_pin(frame_handle: NativeHandle, pane_name: &str, pinned: bool) {
    PANE_HOSTS.with(|m| {
        if let Some(state) = m
            .borrow_mut()
            .get_mut(&(frame_handle, pane_name.to_string()))
        {
            state.pinned = pinned;
            for (_, btn, kind) in &state.buttons {
                if *kind == AuiPaneButton::Pin {
                    let label = to_wide(button_glyph(AuiPaneButton::Pin, pinned));
                    // SAFETY: SetWindowTextW on owned button.
                    unsafe {
                        windows_sys::Win32::UI::WindowsAndMessaging::SetWindowTextW(
                            *btn,
                            label.as_ptr(),
                        );
                    }
                }
            }
        }
    });
}

pub fn float_aui_pane_host(frame_handle: NativeHandle, pane_name: &str, rect: &Rect, title: &str) {
    let Some(state) = PANE_HOSTS.with(|m| {
        m.borrow()
            .get(&(frame_handle, pane_name.to_string()))
            .cloned()
    }) else {
        return;
    };
    ensure_floating_class();
    let title_w = to_wide(title);
    // SAFETY: create floating popup for detached pane.
    let float_hwnd = unsafe {
        CreateWindowExW(
            0,
            to_wide("RuWxAuiPaneFloat").as_ptr(),
            title_w.as_ptr(),
            WS_VISIBLE
                | windows_sys::Win32::UI::WindowsAndMessaging::WS_POPUP
                | windows_sys::Win32::UI::WindowsAndMessaging::WS_CAPTION
                | windows_sys::Win32::UI::WindowsAndMessaging::WS_THICKFRAME
                | windows_sys::Win32::UI::WindowsAndMessaging::WS_SYSMENU,
            rect.x,
            rect.y,
            rect.width as i32,
            rect.height as i32,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            GetModuleHandleW(std::ptr::null()),
            std::ptr::null_mut(),
        )
    };
    if float_hwnd.is_null() {
        return;
    }
    // SAFETY: reparent host into floating window.
    unsafe {
        SetParent(state.host_hwnd, float_hwnd);
        MoveWindow(
            state.host_hwnd,
            0,
            0,
            rect.width as i32,
            rect.height as i32,
            1,
        );
        ShowWindow(state.host_hwnd, SW_SHOW);
    }
    PANE_FLOATS.with(|m| {
        m.borrow_mut()
            .insert((frame_handle, pane_name.to_string()), float_hwnd);
    });
}

pub fn dock_aui_pane_host(frame_handle: NativeHandle, pane_name: &str) {
    let float_hwnd = PANE_FLOATS.with(|m| {
        m.borrow_mut()
            .remove(&(frame_handle, pane_name.to_string()))
    });
    let Some(state) = PANE_HOSTS.with(|m| {
        m.borrow()
            .get(&(frame_handle, pane_name.to_string()))
            .cloned()
    }) else {
        return;
    };
    // SAFETY: reparent host back to frame client area.
    unsafe {
        SetParent(state.host_hwnd, state.parent_handle as HWND);
        if let Some(float_hwnd) = float_hwnd {
            DestroyWindow(float_hwnd);
        }
    }
}

fn ensure_floating_class() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        // SAFETY: register floating pane window class.
        unsafe {
            let class_name = to_wide("RuWxAuiPaneFloat");
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(float_pane_wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: GetModuleHandleW(std::ptr::null()),
                hIcon: std::ptr::null_mut(),
                hCursor: std::ptr::null_mut(),
                hbrBackground: GetStockObject(0) as HBRUSH,
                lpszMenuName: std::ptr::null(),
                lpszClassName: class_name.as_ptr(),
                hIconSm: std::ptr::null_mut(),
            };
            RegisterClassExW(&wc);
        }
    });
}

unsafe extern "system" fn float_pane_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        // Anche i pannelli flottati devono far salire le notifiche dei
        // controlli figli: `DefWindowProcW` le droppava e gli `on_click`
        // registrati sul frame non scattavano nei pane float.
        WM_COMMAND | WM_NOTIFY | WM_HSCROLL | WM_VSCROLL => {
            let parent = GetParent(hwnd);
            if !parent.is_null() {
                SendMessageW(parent, msg, wparam, lparam)
            } else {
                0
            }
        }
        WM_DESTROY => {
            // Find pane by float hwnd and request dock.
            let entry = PANE_FLOATS.with(|m| {
                m.borrow()
                    .iter()
                    .find(|(_, fh)| **fh == hwnd)
                    .map(|(k, _)| k.clone())
            });
            if let Some((frame_handle, pane_name)) = entry {
                PANE_FLOATS.with(|m| {
                    m.borrow_mut().remove(&(frame_handle, pane_name.clone()));
                });
                PANE_ACTION.with(|act| {
                    if let Some((mgr, frame)) = act.borrow().get(&frame_handle).cloned() {
                        unsafe {
                            if let Some(mgr) = mgr.as_mut() {
                                mgr.dock_pane(&frame, &pane_name);
                            }
                        }
                    }
                });
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

const SASH_CLASS: &str = "RuWxAuiSash";

/// In-progress sash drag: `(frame_handle, pane_name, vertical, start_pos, start_size)`.
type SashDragState = (NativeHandle, String, bool, i32, i32);

thread_local! {
    static SASH_WINDOWS: RefCell<HashMap<(NativeHandle, String), HWND>> =
        RefCell::new(HashMap::new());
    static SASH_DRAG: RefCell<Option<SashDragState>> = const { RefCell::new(None) };
}

struct SashDragUserData {
    frame_handle: NativeHandle,
    pane_name: String,
    vertical: bool,
}

fn ensure_sash_class() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| unsafe {
        let class_name = to_wide(SASH_CLASS);
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(sash_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: GetModuleHandleW(std::ptr::null()),
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: GetStockObject(0) as HBRUSH,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wc);
    });
}

unsafe extern "system" fn sash_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_LBUTTONDOWN => {
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let data = &*(ptr as *const SashDragUserData);
                SASH_DRAG.with(|d| {
                    *d.borrow_mut() = Some((
                        data.frame_handle,
                        data.pane_name.clone(),
                        data.vertical,
                        x,
                        y,
                    ));
                });
                SetCapture(hwnd);
            }
            0
        }
        WM_MOUSEMOVE => {
            if wparam & 0x0001 != 0 {
                let x = (lparam & 0xFFFF) as i16 as i32;
                let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
                SASH_DRAG.with(|d| {
                    let mut slot = d.borrow_mut();
                    if let Some((frame, name, vertical, lx, ly)) = slot.as_mut() {
                        let delta = if *vertical { x - *lx } else { y - *ly };
                        if delta != 0 {
                            PANE_ACTION.with(|act| {
                                if let Some((mgr, frame)) = act.borrow().get(frame).cloned() {
                                    if let Some(mgr) = unsafe { mgr.as_mut() } {
                                        mgr.apply_sash_resize(&frame, name, delta, *vertical);
                                    }
                                }
                            });
                            *lx = x;
                            *ly = y;
                        }
                    }
                });
            }
            0
        }
        WM_LBUTTONUP => {
            ReleaseCapture();
            SASH_DRAG.with(|d| d.borrow_mut().take());
            0
        }
        WM_DESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                drop(Box::from_raw(ptr as *mut SashDragUserData));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub fn sync_aui_sashes(parent: NativeHandle, frame_handle: NativeHandle, specs: &[AuiSashSpec]) {
    ensure_sash_class();
    let parent_hwnd = parent as HWND;
    let wanted: std::collections::HashSet<(NativeHandle, String)> = specs
        .iter()
        .map(|s| (frame_handle, s.pane_name.clone()))
        .collect();

    SASH_WINDOWS.with(|m| {
        let mut map = m.borrow_mut();
        map.retain(|key, hwnd| {
            if wanted.contains(key) {
                true
            } else {
                unsafe {
                    DestroyWindow(*hwnd);
                }
                false
            }
        });

        for spec in specs {
            let key = (frame_handle, spec.pane_name.clone());
            if let Some(hwnd) = map.get(&key).copied() {
                unsafe {
                    MoveWindow(
                        hwnd,
                        spec.rect.x,
                        spec.rect.y,
                        spec.rect.width as i32,
                        spec.rect.height as i32,
                        1,
                    );
                    ShowWindow(hwnd, SW_SHOW);
                }
                continue;
            }
            let hwnd = unsafe {
                CreateWindowExW(
                    0,
                    to_wide(SASH_CLASS).as_ptr(),
                    std::ptr::null(),
                    WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
                    spec.rect.x,
                    spec.rect.y,
                    spec.rect.width as i32,
                    spec.rect.height as i32,
                    parent_hwnd,
                    std::ptr::null_mut(),
                    GetModuleHandleW(std::ptr::null()),
                    std::ptr::null_mut(),
                )
            };
            if hwnd.is_null() {
                continue;
            }
            let data = Box::new(SashDragUserData {
                frame_handle,
                pane_name: spec.pane_name.clone(),
                vertical: spec.vertical,
            });
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(data) as isize);
            }
            map.insert(key, hwnd);
        }
    });
}
