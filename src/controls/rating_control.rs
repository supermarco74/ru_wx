//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! RatingControl (`wxRatingCtrl`) — stelle con mezzo step (0.0 … max).
//!
//! # API
//!
//! * [`RatingControl::new`] — crea il controllo con `max_stars` stelle
//!   (minimo 1);
//! * [`RatingControl::get_value`] / [`RatingControl::set_value`] —
//!   legge/imposta il valore (clamp `0..=max`, snap a passo 0.5);
//! * [`RatingControl::set_max_stars`] — cambia il numero di stelle
//!   (ridimensiona il controllo);
//! * [`RatingControl::on_change`] — callback invocata al clic su una
//!   stella (o mezza stella);
//! * [`RatingControl::default_size`] — dimensione naturale per i sizer.
//!
//! # Win32: classe custom + owner-draw (nota §5 DIRETTIVE_PROGETTO)
//!
//! Il controllo è una finestra figlia della classe custom
//! `RuWxRatingCtrl` (registrata alla prima costruzione con
//! `RegisterClassExW`, `CS_HREDRAW | CS_VREDRAW`, sfondo `COLOR_WINDOW`).
//!
//! Le stelle sono **owner-draw**: non esiste una classe nativa Win32 che
//! disegni stelle con mezzo step (le common controls hanno solo la
//! `PBS_SMOOTH` della progress bar), quindi il tema di sistema non
//! "basta" e il disegno manuale è esattamente il caso previsto dalla §5
//! ("owner-draw solo se il tema nativo non basta"). Ogni stella è un
//! glifo `U+2605` ★ (piena) / `U+2606` ☆ (vuota) disegnato con
//! `DrawTextW`; la mezza stella si ottiene ridisegnando ★ dentro un
//! clip rect limitato alla metà sinistra della cella
//! (`SaveDC` / `IntersectClipRect` / `RestoreDC`).
//!
//! Nessun `SetWindowTheme`: il tema di sistema (Windows 11) resta
//! attivo, il colore pieno è l'accent di sistema da
//! [`crate::core::tokens::system_accent_colour`], le stelle vuote
//! usano `GetSysColor(COLOR_GRAYTEXT)`. Il font arriva da `WM_GETFONT`
//! con fallback `DEFAULT_GUI_FONT`. Nessuna dipendenza aggiuntiva in
//! `Cargo.toml`.
//!
//! # Input
//!
//! * `WM_LBUTTONDOWN` — metà sinistra della stella *i* → `i` stelle
//!   (step `.0`), metà destra → `i + 0.5` (step `.5`), clamp `0..=max`;
//! * `WM_MOUSEMOVE` / `WM_MOUSELEAVE` (`TrackMouseEvent` + `TME_LEAVE`)
//!   — evidenzia all'hover la stella sotto il cursore;
//! * nessun handler sul [`crate::window::frame::Frame`]: la callback di
//!   [`RatingControl::on_change`] è custodita nel `RefCell` interno e
//!   invocata dal wndproc (stesso routing di `BreadcrumbBar`).
//!
//! # Stub non-Windows
//!
//! Su non-Windows il tipo resta costruibile (handle finto + rect) e i
//! getter/setter funzionano sullo stato, così il codice client compila
//! cross-platform.

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::geometry::Rect;
use crate::core::tokens::SPACING_XS;
use crate::core::widget::{Widget, WidgetRef};
use crate::window::frame::Frame;

use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, DrawTextW, EndPaint, FillRect, GetStockObject, GetSysColor, IntersectClipRect,
    InvalidateRect, RestoreDC, SaveDC, SelectObject, SetBkMode, SetTextColor, COLOR_GRAYTEXT,
    COLOR_WINDOW, DEFAULT_GUI_FONT, DT_CENTER, DT_SINGLELINE, DT_VCENTER, HBRUSH, HDC, PAINTSTRUCT,
    TRANSPARENT,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Lato di una cella stella in pixel (fascia 16–18 richiesta dalla spec).
const STAR_SIZE: i32 = 16;

/// Spazio orizzontale tra due celle stella.
const STAR_GAP: i32 = SPACING_XS;

/// Margine interno attorno alle stelle (`SPACING_XS` da `tokens`).
const PADDING: i32 = SPACING_XS;

/// Glifo stella piena (`U+2605`).
const STAR_FULL: &str = "\u{2605}";

/// Glifo stella vuota (`U+2606`).
const STAR_EMPTY: &str = "\u{2606}";

struct RatingControlInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(not(target_os = "windows"))]
    stub_handle: isize,
    rect: Rect,
    /// Numero massimo di stelle (almeno 1).
    max_stars: u32,
    /// Valore corrente `0.0 ..= max_stars`, sempre multipla di 0.5.
    value: f32,
    /// Indice della stella sotto il cursore (evidenziazione hover).
    hover: Option<u32>,
    /// `true` tra il primo `WM_MOUSEMOVE` e il `WM_MOUSELEAVE`.
    mouse_tracking: bool,
    /// Flag anti-loop: `true` mentre la callback è in esecuzione.
    updating: bool,
    /// Callback registrata con [`RatingControl::on_change`].
    handler: Option<Box<dyn Fn(f32)>>,
    visible: bool,
    enabled: bool,
}

#[derive(Clone)]
pub struct RatingControl {
    inner: Rc<RefCell<RatingControlInner>>,
}

/// Dimensione naturale per `max_stars` stelle: due margini, `n` celle e
/// `n - 1` gap. Con i valori correnti l'altezza è 24 px (fascia 20–24).
fn size_for_stars(max_stars: u32) -> (u32, u32) {
    let n = max_stars.max(1) as i32;
    let w = 2 * PADDING + n * STAR_SIZE + (n - 1) * STAR_GAP;
    let h = 2 * PADDING + STAR_SIZE;
    (w as u32, h as u32)
}

/// Clamp `0 ..= max_stars` + snap al mezzo step più vicino.
fn clamp_value(v: f32, max_stars: u32) -> f32 {
    let max = max_stars.max(1) as f32;
    let v = v.clamp(0.0, max);
    (v * 2.0).round() / 2.0
}

/// Indice della stella sotto la coordinata `x` e se il clic è caduto
/// nella metà destra (`.5`); oltre l'ultima stella viene ristretta
/// all'ultima cella.
#[cfg(target_os = "windows")]
fn slot_at(x: i32, max_stars: u32) -> (u32, bool) {
    let n = max_stars.max(1);
    let rel = (x - PADDING).max(0);
    let slot = STAR_SIZE + STAR_GAP;
    let idx = rel / slot;
    if idx as u32 >= n {
        return (n - 1, true);
    }
    (idx as u32, rel - idx * slot >= STAR_SIZE / 2)
}

/// Valore risultante da un clic in `x`: metà sinistra stella *i* →
/// `i + 1.0`, metà destra → `i + 1.5`, poi clamp `0 ..= max_stars`.
#[cfg(target_os = "windows")]
fn value_at(x: i32, max_stars: u32) -> f32 {
    let (idx, right) = slot_at(x, max_stars);
    let v = idx as f32 + 1.0 + if right { 0.5 } else { 0.0 };
    clamp_value(v, max_stars)
}

/// Indice della stella sotto la coordinata `x`, `None` fuori dall'area
/// stelle (gap compresi, assegna alla stella precedente).
#[cfg(target_os = "windows")]
fn hover_at(x: i32, max_stars: u32) -> Option<u32> {
    let n = max_stars.max(1) as i32;
    if x < PADDING {
        return None;
    }
    let rel = x - PADDING;
    let area = n * STAR_SIZE + (n - 1) * STAR_GAP;
    if rel >= area {
        return None;
    }
    Some((rel / (STAR_SIZE + STAR_GAP)).min(n - 1) as u32)
}

/// Registra la classe `RuWxRatingCtrl` (idempotente: la seconda
/// chiamata restituisce `ERROR_CLASS_ALREADY_EXISTS`, che va ignorata).
#[cfg(target_os = "windows")]
fn register_rating_class() {
    // SAFETY: `WNDCLASSEXW` compilato a mano con campi tutti validi;
    // l'instance è l'handle di modulo corrente.
    unsafe {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_name = to_wide("RuWxRatingCtrl");

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(rating_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: (COLOR_WINDOW + 1) as HBRUSH,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wc);
    }
}

/// Recupera lo `Rc<RefCell<...>>` stoccato in `GWLP_USERDATA`.
///
/// `Rc::into_raw` in [`RatingControl::new`] lascia il conteggio a 1;
/// qui va **prima** incrementato e poi riconvertito con `from_raw`,
/// sennò il secondo dispatch dropperebbe il conteggio a 0 e libererebbe
/// la memoria (stesso rationale dei commenti in `frame.rs` WM_NOTIFY).
#[cfg(target_os = "windows")]
unsafe fn state_of(hwnd: HWND) -> Option<Rc<RefCell<RatingControlInner>>> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
    if ptr == 0 {
        return None;
    }
    Rc::increment_strong_count(ptr as *const RefCell<RatingControlInner>);
    Some(Rc::from_raw(ptr as *const RefCell<RatingControlInner>))
}

/// Disegna un glifo stella centrato nella cella `cell`.
#[cfg(target_os = "windows")]
fn draw_star(hdc: HDC, cell: RECT, glyph: &str, colour: u32) {
    // SAFETY: `hdc` è il DC di paint valido per tutta la chiamata;
    // `wide` vive fino a `DrawTextW`.
    unsafe {
        SetTextColor(hdc, colour);
        let wide = to_wide(glyph);
        let mut rc = cell;
        DrawTextW(
            hdc,
            wide.as_ptr(),
            (wide.len() - 1) as i32,
            &mut rc,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );
    }
}

/// Disegna l'intero controllo (sfondo + stelle) sul DC di paint.
#[cfg(target_os = "windows")]
fn paint_control(hwnd: HWND, hdc: HDC) {
    // SAFETY: chiamata solo da `WM_PAINT` con il `hdc` di `BeginPaint`.
    unsafe {
        let mut client: RECT = std::mem::zeroed();
        GetClientRect(hwnd, &mut client);
        // Sfondo `COLOR_WINDOW` + `WM_ERASEBKGND => 1`: un solo fill,
        // nessun flicker.
        FillRect(hdc, &client, (COLOR_WINDOW + 1) as HBRUSH);

        // Copia dello stato senza tenere il `borrow` attivo durante le
        // chiamate GDI (il `Rc` viene droppato alla fine del `map`).
        let state = state_of(hwnd).map(|rc| {
            let inner = rc.borrow();
            (inner.value, inner.hover, inner.max_stars, inner.enabled)
        });
        let (value, hover, max_stars, enabled) = match state {
            Some(s) => s,
            None => return,
        };

        // Font del controllo (`WM_GETFONT`) o font GUI di sistema.
        let hfont = SendMessageW(hwnd, WM_GETFONT, 0, 0);
        let old_font = if hfont != 0 {
            SelectObject(hdc, hfont as _)
        } else {
            SelectObject(hdc, GetStockObject(DEFAULT_GUI_FONT))
        };
        let old_bk = SetBkMode(hdc, TRANSPARENT as i32);

        // Stelle piene = accent di sistema; vuote = grigio di sistema;
        // disabilitato = tutto grigio.
        let empty = GetSysColor(COLOR_GRAYTEXT);
        let accent = if enabled {
            crate::core::tokens::system_accent_colour().to_colorref()
        } else {
            empty
        };

        for i in 0..max_stars {
            let left = PADDING + (i as i32) * (STAR_SIZE + STAR_GAP);
            let cell = RECT {
                left,
                top: PADDING,
                right: left + STAR_SIZE,
                bottom: PADDING + STAR_SIZE,
            };
            if value >= (i as f32) + 1.0 {
                // Stella piena.
                draw_star(hdc, cell, STAR_FULL, accent);
            } else {
                // Stella vuota (accent se sotto il cursore).
                let base = if hover == Some(i) { accent } else { empty };
                draw_star(hdc, cell, STAR_EMPTY, base);
                if value >= (i as f32) + 0.5 {
                    // Mezza stella: ★ limitato alla metà sinistra.
                    let saved = SaveDC(hdc);
                    IntersectClipRect(
                        hdc,
                        cell.left,
                        cell.top,
                        cell.left + STAR_SIZE / 2,
                        cell.bottom,
                    );
                    draw_star(hdc, cell, STAR_FULL, accent);
                    RestoreDC(hdc, saved);
                }
            }
        }

        SetBkMode(hdc, old_bk);
        SelectObject(hdc, old_font);
    }
}

/// Invoca la callback registrata con [`RatingControl::on_change`].
///
/// La callback viene `take()`ta prima dell'invocazione (nessun `borrow`
/// attivo: può chiamare `set_value` in sicurezza) e ripristinata dopo,
/// a meno che non ne sia stata installata una nuova al suo interno. Il
/// flag `updating` ignora le notifiche rientranti (anti-loop).
#[cfg(target_os = "windows")]
fn notify_change(inner: &Rc<RefCell<RatingControlInner>>) {
    let (value, handler) = {
        let mut b = inner.borrow_mut();
        if b.updating {
            return;
        }
        b.updating = true;
        (b.value, b.handler.take())
    };
    if let Some(handler) = handler.as_ref() {
        handler(value);
    }
    let mut b = inner.borrow_mut();
    if b.handler.is_none() {
        b.handler = handler;
    }
    b.updating = false;
}

/// WndProc della classe `RuWxRatingCtrl`.
#[cfg(target_os = "windows")]
unsafe extern "system" fn rating_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            paint_control(hwnd, hdc);
            EndPaint(hwnd, &ps);
            0
        }
        // Il background lo dipinge `paint_control` con un solo `FillRect`.
        WM_ERASEBKGND => 1,
        WM_LBUTTONDOWN => {
            // `GET_X_LPARAM` non è esposto da windows-sys 0.59: decodifica
            // manuale della coordinata (stesso pattern di `splitter_window`).
            let x = (lparam & 0xFFFF) as i16 as i32;
            let rc = match state_of(hwnd) {
                Some(rc) => rc,
                None => return DefWindowProcW(hwnd, msg, wparam, lparam),
            };
            let changed = {
                let mut inner = rc.borrow_mut();
                if !inner.enabled {
                    return 0;
                }
                let v = value_at(x, inner.max_stars);
                if (v - inner.value).abs() < f32::EPSILON {
                    false
                } else {
                    inner.value = v;
                    true
                }
            };
            if changed {
                InvalidateRect(hwnd, std::ptr::null(), 1);
                notify_change(&rc);
            }
            0
        }
        WM_MOUSEMOVE => {
            let x = (lparam & 0xFFFF) as i16 as i32;
            if let Some(rc) = state_of(hwnd) {
                let (changed, need_track) = {
                    let mut inner = rc.borrow_mut();
                    let h = if inner.enabled {
                        hover_at(x, inner.max_stars)
                    } else {
                        None
                    };
                    let changed = inner.hover != h;
                    inner.hover = h;
                    let need_track = !inner.mouse_tracking;
                    inner.mouse_tracking = need_track;
                    (changed, need_track)
                };
                if need_track {
                    let mut tme = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    TrackMouseEvent(&mut tme);
                }
                if changed {
                    InvalidateRect(hwnd, std::ptr::null(), 1);
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        // `WM_MOUSELEAVE` non è esposto da `WindowsAndMessaging` in
        // windows-sys 0.59 (vale 675 = 0x2A3): letterale come in `frame.rs`.
        0x02A3 => {
            if let Some(rc) = state_of(hwnd) {
                let had_hover = {
                    let mut inner = rc.borrow_mut();
                    inner.mouse_tracking = false;
                    inner.hover.take().is_some()
                };
                if had_hover {
                    InvalidateRect(hwnd, std::ptr::null(), 1);
                }
            }
            0
        }
        WM_DESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let _ = Rc::from_raw(ptr as *const RefCell<RatingControlInner>);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
impl RatingControl {
    /// Crea un rating con `max_stars` stelle (minimo 1) come figlio di
    /// `parent`.
    ///
    /// Il numero di stelle definisce anche la dimensione naturale
    /// (vedi [`RatingControl::default_size`]): `8 + 16·n + 4·(n-1)`
    /// per `24` pixel di altezza.
    pub fn new(parent: &dyn Widget, max_stars: u32) -> Self {
        let max_stars = max_stars.max(1);
        let id = next_control_id();
        let (w, h) = size_for_stars(max_stars);

        #[cfg(target_os = "windows")]
        register_rating_class();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let wide_class = to_wide("RuWxRatingCtrl");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE,
                0,
                0,
                w as i32,
                h as i32,
                parent.native_handle() as HWND,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = (parent, id);

        let ctrl = RatingControl {
            inner: Rc::new(RefCell::new(RatingControlInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(not(target_os = "windows"))]
                stub_handle: crate::platform::stub_backend::alloc_widget_handle(),
                rect: Rect::new(0, 0, w, h),
                max_stars,
                value: 0.0,
                hover: None,
                mouse_tracking: false,
                updating: false,
                handler: None,
                visible: true,
                enabled: true,
            })),
        };

        // Stato per il wndproc: un clone con `Rc::into_raw` (il
        // conteggio sale a 2) + `GWLP_USERDATA`; il wndproc lo
        // riconverte con `increment_strong_count` + `from_raw` e lo
        // rilascia in `WM_DESTROY`.
        #[cfg(target_os = "windows")]
        {
            let raw = Rc::into_raw(ctrl.inner.clone());
            // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw as isize);
            }
        }

        ctrl
    }

    /// Valore corrente `0.0 ..= max_stars`, sempre multipla di 0.5.
    pub fn get_value(&self) -> f32 {
        self.inner.borrow().value
    }

    /// Imposta il valore: clamp `0 ..= max_stars` + snap al mezzo step.
    ///
    /// Come in `wxRatingCtrl`, il set programmatico **non** invoca la
    /// callback di [`RatingControl::on_change`].
    pub fn set_value(&self, v: f32) {
        let mut inner = self.inner.borrow_mut();
        let v = clamp_value(v, inner.max_stars);
        if (v - inner.value).abs() < f32::EPSILON {
            return;
        }
        inner.value = v;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            InvalidateRect(inner.hwnd, std::ptr::null(), 1);
        }
    }

    /// Cambia il numero di stelle (minimo 1): il valore corrente viene
    /// ricalcolato sul nuovo massimo e il controllo ridimensionato.
    pub fn set_max_stars(&self, n: u32) {
        let n = n.max(1);
        let mut inner = self.inner.borrow_mut();
        inner.max_stars = n;
        inner.value = clamp_value(inner.value, n);
        inner.hover = None;
        let (w, h) = size_for_stars(n);
        inner.rect.width = w;
        inner.rect.height = h;
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            MoveWindow(
                inner.hwnd,
                inner.rect.x,
                inner.rect.y,
                w as i32,
                h as i32,
                1,
            );
            InvalidateRect(inner.hwnd, std::ptr::null(), 1);
        }
    }

    /// Registra la callback `cb(nuovo_valore)`, invocata quando
    /// l'utente clicca su una stella (o mezza stella).
    ///
    /// Il parametro `frame` è richiesto dalla firma dell'API (coerenza
    /// con `SearchCtrl::on_change` / `BreadcrumbBar::on_navigate`): il
    /// routing del clic è gestito dal wndproc della classe custom,
    /// quindi nessun handler va registrato sul frame.
    pub fn on_change(&self, _frame: &Frame, cb: impl Fn(f32) + 'static) {
        self.inner.borrow_mut().handler = Some(Box::new(cb));
    }

    /// Dimensione naturale `(larghezza, altezza)` in pixel per il
    /// numero corrente di stelle (da passare ai sizer).
    pub fn default_size(&self) -> (u32, u32) {
        size_for_stars(self.inner.borrow().max_stars)
    }

    /// WidgetRef per i sizer.
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

impl Widget for RatingControlInner {
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
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.hover = None;
        }
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            EnableWindow(self.hwnd, if enabled { 1 } else { 0 });
            InvalidateRect(self.hwnd, std::ptr::null(), 1);
        }
    }
}
