//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! TeachingTip (`TeachingTip` WinUI) — callout con freccia, titolo,
//! corpo testo e pulsante di chiusura "×".
//!
//! # API
//!
//! * [`TeachingTip::new`] — crea il callout (nascosto);
//! * [`TeachingTip::show_for`] — lo posiziona sotto (o sopra, se non
//!   c'è spazio) il widget target e lo mostra senza rubare il focus;
//! * [`TeachingTip::hide`] / [`TeachingTip::is_visible`] — nasconde e
//!   interroga lo stato;
//! * [`TeachingTip::set_title`] / [`TeachingTip::set_body`] — cambiano
//!   i testi (e ridimensionano il callout se visibile);
//! * [`TeachingTip::on_close`] — callback invocata quando il callout
//!   viene nascosto da ×, `Esc` o perdita di attivazione;
//! * [`TeachingTip::default_size`] — dimensione naturale per i sizer.
//!
//! # Win32: popup classe custom (nota §5 DIRETTIVE_PROGETTO)
//!
//! Il callout è una finestra top-level della classe custom
//! `RuWxTeachingTip` (registrata alla prima costruzione):
//! `WS_POPUP` + `WS_EX_TOOLWINDOW | WS_EX_TOPMOST`, sfondo disegnato in
//! `WM_PAINT` (`WM_ERASEBKGND => 1`, nessun flicker). Nessun
//! `SetWindowTheme`: i colori arrivano dal tema di sistema
//! (`GetSysColor(COLOR_WINDOW/COLOR_GRAYTEXT)`) come previsto dalla §5.
//!
//! Il pannello è bordato 1 px con la freccia triangolare (`Polygon`)
//! verso il target; titolo e corpo sono due `STATIC` figlie con font
//! `Segoe UI` (titolo grassetto via [`crate::core::font::FontDesc::bold`]).
//! Il glifo di chiusura `✕` è disegnato in owner-draw: non esiste una
//! classe nativa Win32 per il callout, quindi il disegno manuale è il
//! caso previsto dalla §5 ("owner-draw solo se il tema nativo non
//! basta").
//!
//! # Comportamento
//!
//! * `show_for` registra (una sola volta) gli hook `on_move_event` /
//!   `on_resize` / `on_hide` del [`crate::window::frame::Frame`]:
//!   il callout segue il frame e viene nascosto quando il frame lo è;
//! * `WM_KEYDOWN VK_ESCAPE` / clic su `✕` / `WM_ACTIVATE` con
//!   `WA_INACTIVE` → nascondi + [`TeachingTip::on_close`];
//! * posizionamento sotto il target con fallback sopra
//!   (`GetWindowRect` + `GetSystemMetrics(SM_CXSCREEN/SM_CYSCREEN)`).
//!
//! # Esempio
//! ```no_run
//! use ru_wx::prelude::*;
//!
//! let frame = Frame::builder().with_title("Demo").with_size(600, 400).build();
//! let button = Button::new(&frame, "Salva");
//! let tip = TeachingTip::new("Salvato", "Il record è stato aggiornato.");
//! let target = button.as_widget_ref();
//! tip.show_for(&*target.borrow(), &frame);
//! // ... dopo un po':
//! tip.hide();
//! ```

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::tokens::{SPACING_S, SPACING_XS};
use crate::core::widget::Widget;
use crate::window::frame::Frame;

#[cfg(target_os = "windows")]
use crate::controls::static_text::measure_text_size;
#[cfg(target_os = "windows")]
use crate::core::font::{Font, FontDesc};
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, CreatePen, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint, FillRect,
    FrameRect, GetDC, GetStockObject, GetSysColor, InvalidateRect, Polygon, ReleaseDC,
    SelectObject, SetBkMode, SetTextColor, UpdateWindow, COLOR_GRAYTEXT, COLOR_WINDOW, DT_CALCRECT,
    DT_CENTER, DT_SINGLELINE, DT_VCENTER, DT_WORDBREAK, HBRUSH, HDC, HFONT, NULL_BRUSH,
    PAINTSTRUCT, PS_SOLID, TRANSPARENT,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Nome della classe finestra del callout.
#[cfg(target_os = "windows")]
const TIP_CLASS_NAME: &str = "RuWxTeachingTip";

/// Altezza della freccia triangolare (px).
const ARROW: i32 = 8;
/// Mezza base della freccia (px).
const ARR_HALF: i32 = 8;
/// Margine interno del pannello (`SPACING_S` = 8).
const PAD: i32 = SPACING_S;
/// Gap tra titolo e corpo (`SPACING_XS` = 4).
const GAP: i32 = SPACING_XS;
/// Larghezza della zona del glifo `✕` (px).
const CLOSE_ZONE: i32 = 20;
/// Distanza del callout dal target (px).
const TARGET_GAP: i32 = 6;
/// Larghezza minima del callout (px).
const MIN_W: i32 = 240;
/// Larghezza massima del callout (px).
const MAX_W: i32 = 420;
/// Larghezza predefinita (per `default_size`).
const DEFAULT_W: i32 = 300;
/// Altezza predefinita (per `default_size`).
const DEFAULT_H: i32 = 96;

struct TeachingTipInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(target_os = "windows")]
    title_hwnd: HWND,
    #[cfg(target_os = "windows")]
    body_hwnd: HWND,
    #[cfg(target_os = "windows")]
    title_font: Font,
    #[cfg(target_os = "windows")]
    body_font: Font,
    /// Widget target dell'ultimo [`TeachingTip::show_for`].
    #[cfg(target_os = "windows")]
    target: Option<HWND>,
    /// Rettangolo di hit-test/disegno del glifo `✕` (client space).
    #[cfg(target_os = "windows")]
    close_rect: RECT,
    /// `true` quando il callout sta sotto il target (freccia in alto).
    #[cfg(target_os = "windows")]
    arrow_up: bool,
    /// Ascissa del vertice della freccia (client space).
    #[cfg(target_os = "windows")]
    arrow_ax: i32,
    /// Dimensione corrente del callout `(w, h)`.
    #[cfg(target_os = "windows")]
    size: (i32, i32),
    #[cfg(not(target_os = "windows"))]
    stub_handle: isize,
    title: String,
    body: String,
    visible: bool,
    /// Flag anti-loop: `true` durante `ShowWindow(SW_HIDE)`.
    hiding: bool,
    /// `true` dopo la registrazione degli hook del frame (una volta).
    hooked: bool,
    /// Callback registrata con [`TeachingTip::on_close`].
    on_close: Option<Box<dyn FnMut()>>,
}

#[derive(Clone)]
pub struct TeachingTip {
    inner: Rc<RefCell<TeachingTipInner>>,
}

// ── Registrazione classe + creazione ──────────────────────────────────

/// Registra la classe `RuWxTeachingTip` (idempotente: la seconda
/// chiamata restituisce `ERROR_CLASS_ALREADY_EXISTS`, che va ignorata).
#[cfg(target_os = "windows")]
fn register_tip_class() {
    // SAFETY: `WNDCLASSEXW` compilato a mano con campi tutti validi;
    // l'instance è l'handle di modulo corrente. `NULL_BRUSH` come
    // sfondo: il pannello e la freccia sono disegnati in `WM_PAINT`.
    unsafe {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_name = to_wide(TIP_CLASS_NAME);

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(teaching_tip_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: GetStockObject(NULL_BRUSH) as HBRUSH,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wc);
    }
}

/// Crea la finestra popup nascosta con le due `STATIC` figlie e i font.
///
/// Ritorna `(hwnd, title_hwnd, body_hwnd, title_font, body_font)`.
#[cfg(target_os = "windows")]
unsafe fn create_tip_window(title: &str, body: &str) -> (HWND, HWND, HWND, Font, Font) {
    register_tip_class();

    // SAFETY: Win32 FFI con argomenti validi; le stringhe wide vivono
    // fino a `CreateWindowExW`.
    unsafe {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let wide_class = to_wide(TIP_CLASS_NAME);

        // Popup top-level, non attivato (nascosto) alla creazione.
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            wide_class.as_ptr(),
            std::ptr::null(),
            WS_POPUP,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            DEFAULT_W,
            DEFAULT_H,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null_mut(),
        );

        // Font: titolo grassetto, corpo regolare (entrambi Segoe UI).
        let title_font = Font::new(FontDesc::new("Segoe UI", 11).bold());
        let body_font = Font::new(FontDesc::new("Segoe UI", 11));

        let wide_title = to_wide(title);
        let title_hwnd = CreateWindowExW(
            0,
            to_wide("STATIC").as_ptr(),
            wide_title.as_ptr(),
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            0,
            0,
            hwnd,
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null_mut(),
        );
        SendMessageW(title_hwnd, WM_SETFONT, title_font.hfont() as usize, 1);

        let wide_body = to_wide(body);
        let body_hwnd = CreateWindowExW(
            0,
            to_wide("STATIC").as_ptr(),
            wide_body.as_ptr(),
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            0,
            0,
            hwnd,
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null_mut(),
        );
        SendMessageW(body_hwnd, WM_SETFONT, body_font.hfont() as usize, 1);

        (hwnd, title_hwnd, body_hwnd, title_font, body_font)
    }
}

/// Recupera lo `Rc<RefCell<...>>` stoccato in `GWLP_USERDATA`.
///
/// `Rc::into_raw` in [`TeachingTip::new`] lascia il conteggio a 2; qui
/// va **prima** incrementato e poi riconvertito con `from_raw`, sennò il
/// secondo dispatch dropperebbe il conteggio (stesso rationale dei
/// commenti in `frame.rs` WM_NOTIFY).
#[cfg(target_os = "windows")]
unsafe fn state_of(hwnd: HWND) -> Option<Rc<RefCell<TeachingTipInner>>> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
    if ptr == 0 {
        return None;
    }
    Rc::increment_strong_count(ptr as *const RefCell<TeachingTipInner>);
    Some(Rc::from_raw(ptr as *const RefCell<TeachingTipInner>))
}

// ── Layout / posizionamento ───────────────────────────────────────────

/// Altezza del corpo testo con word-wrap su `width` px.
#[cfg(target_os = "windows")]
fn measure_wrapped_height(hwnd: HWND, font: HFONT, text: &str, width: i32) -> i32 {
    if text.is_empty() {
        return 0;
    }
    // SAFETY: `GetDC`/`ReleaseDC` sulla finestra viva; il vecchio font
    // viene ripristinato in ogni percorso.
    unsafe {
        let hdc = GetDC(hwnd);
        if hdc.is_null() {
            return 20;
        }
        let old = SelectObject(hdc, font as _);
        let wide = to_wide(text);
        let mut rc: RECT = RECT {
            left: 0,
            top: 0,
            right: width.max(1),
            bottom: 0,
        };
        DrawTextW(
            hdc,
            wide.as_ptr(),
            (wide.len() - 1) as i32,
            &mut rc,
            DT_CALCRECT | DT_WORDBREAK,
        );
        SelectObject(hdc, old);
        ReleaseDC(hwnd, hdc);
        (rc.bottom - rc.top).max(16)
    }
}

/// Posizione e dimensione del callout per un target `tr`.
///
/// Ritorna `(x, y, w, h, arrow_up)` in coordinate schermo.
#[cfg(target_os = "windows")]
fn place_for(tr: &RECT, w: i32, h: i32) -> (i32, i32, i32, i32, bool) {
    // SAFETY: `GetSystemMetrics` senza argomenti validi richiesti.
    unsafe {
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let x = tr.left;
        // Preferenza: sotto il target (freccia in alto).
        if tr.bottom + TARGET_GAP + h <= screen_h {
            (x, tr.bottom + TARGET_GAP, w, h, true)
        } else if tr.top - TARGET_GAP - h >= 0 {
            // Non c'è spazio sotto: sopra il target (freccia in basso).
            (x, tr.top - TARGET_GAP, w, h, false)
        } else {
            // Neanche sopra: al fondo schermo, freccia in alto.
            (x, (screen_h - h).max(0), w, h, true)
        }
    }
}

/// Ridimensiona e riposiziona il callout sul target corrente.
///
/// Se il target è stato distrutto nasconde il callout. Non tocca
/// [`TeachingTipInner::visible`]: a quel ci pensa il chiamante.
#[cfg(target_os = "windows")]
fn place(rc: &Rc<RefCell<TeachingTipInner>>) {
    // ── Fase 1: snapshot + misure (un solo borrow, chiuso subito) ──
    let (hwnd, target, title, body, title_hwnd, body_hwnd, title_font, body_font) = {
        let b = rc.borrow();
        (
            b.hwnd,
            b.target,
            b.title.clone(),
            b.body.clone(),
            b.title_hwnd,
            b.body_hwnd,
            b.title_font.hfont(),
            b.body_font.hfont(),
        )
    };
    if hwnd.is_null() {
        return;
    }
    let Some(target) = target else { return };
    // SAFETY: `IsWindow`/`GetWindowRect` con handle preso dallo stato.
    if unsafe { IsWindow(target) } == 0 {
        hide_tip(rc);
        return;
    }
    let mut tr: RECT = unsafe { std::mem::zeroed() };
    if unsafe { GetWindowRect(target, &mut tr) } == 0 {
        return;
    }

    // Misure del testo (font via `WM_GETFONT` delle figlie).
    let (tw, th) = measure_text_size(title_hwnd, &title);
    let w = ((tw as i32) + 2 * PAD + CLOSE_ZONE).clamp(MIN_W, MAX_W);
    let body_w = w - 2 * PAD;
    let body_h = measure_wrapped_height(body_hwnd, body_font, &body, body_w);
    let h = (ARROW + PAD + th as i32 + GAP + body_h + PAD).max(ARROW + 48);

    let (x, y, w, h, arrow_up) = place_for(&tr, w, h);
    let content_top = if arrow_up { ARROW } else { 0 };
    // Vertice della freccia: centro del target, ristretto ai margini.
    let center = ((tr.left + tr.right) / 2 - x).clamp(ARR_HALF + 2, w - ARR_HALF - 2);
    let close_rect = RECT {
        left: w - PAD - CLOSE_ZONE,
        top: content_top + PAD,
        right: w - PAD,
        bottom: content_top + PAD + th as i32,
    };

    // ── Fase 2: applica le geometrie (nessun borrow attivo) ──
    // SAFETY: handle validi presi dallo stato; `SWP_NOACTIVATE`
    // mantiene il focus sul controllo sottostante.
    unsafe {
        let flags = SWP_NOZORDER | SWP_NOACTIVATE;
        SetWindowPos(
            title_hwnd,
            std::ptr::null_mut(),
            PAD,
            content_top + PAD,
            (w - 2 * PAD - CLOSE_ZONE).max(1),
            th as i32,
            flags,
        );
        SetWindowPos(
            body_hwnd,
            std::ptr::null_mut(),
            PAD,
            content_top + PAD + th as i32 + GAP,
            body_w.max(1),
            body_h,
            flags,
        );
        SetWindowPos(hwnd, std::ptr::null_mut(), x, y, w, h, flags);
    }
    let _ = title_font;

    // ── Fase 3: aggiorna lo stato ──
    {
        let mut b = rc.borrow_mut();
        b.arrow_up = arrow_up;
        b.arrow_ax = center;
        b.close_rect = close_rect;
        b.size = (w, h);
    }
    // SAFETY: `InvalidateRect` su finestra viva.
    unsafe { InvalidateRect(hwnd, std::ptr::null(), 1) };
}

/// Nasconde il callout e invoca la callback [`TeachingTip::on_close`].
fn hide_tip(rc: &Rc<RefCell<TeachingTipInner>>) {
    #[cfg(target_os = "windows")]
    {
        // Fuori dal borrow: `ShowWindow` può rientrare nel wndproc
        // (`WM_ACTIVATE WA_INACTIVE`), che a sua volta legge lo stato.
        let hwnd = {
            let mut b = rc.borrow_mut();
            if !b.visible || b.hiding {
                return;
            }
            b.hiding = true;
            b.visible = false;
            b.hwnd
        };
        // SAFETY: handle preso dallo stato, finestra viva.
        unsafe { ShowWindow(hwnd, SW_HIDE) };
        rc.borrow_mut().hiding = false;
    }

    // Callback fuori da ogni borrow (riposizionabile con `take`).
    let mut cb = rc.borrow_mut().on_close.take();
    if let Some(f) = cb.as_mut() {
        f();
    }
    rc.borrow_mut().on_close = cb;
}

// ── Pittura ───────────────────────────────────────────────────────────

/// Disegna pannello, bordo, freccia e glifo `✕` sul DC di paint.
///
/// SAFETY: chiamato solo da `WM_PAINT` con l'hdc di `BeginPaint`.
#[cfg(target_os = "windows")]
unsafe fn paint_tip(hwnd: HWND, hdc: HDC, rc: &Rc<RefCell<TeachingTipInner>>) {
    // Snapshot: nessun borrow attivo durante le chiamate GDI.
    let (arrow_up, arrow_ax, w, h, close, body_font) = {
        let b = rc.borrow();
        (
            b.arrow_up,
            b.arrow_ax,
            b.size.0,
            b.size.1,
            b.close_rect,
            b.body_font.hfont(),
        )
    };

    // SAFETY: hdc valido per tutta la chiamata; ogni GDI object
    // creato qui viene selezionato via, usato e distrutto.
    unsafe {
        let mut client: RECT = std::mem::zeroed();
        GetClientRect(hwnd, &mut client);

        // Sfondo uniforme (finestra + freccia): un solo fill.
        FillRect(hdc, &client, (COLOR_WINDOW + 1) as HBRUSH);

        // Pannello (sotto la freccia se `arrow_up`, altrimenti sopra).
        let panel = RECT {
            left: 0,
            top: if arrow_up { ARROW } else { 0 },
            right: w,
            bottom: if arrow_up { h } else { h - ARROW },
        };
        let gray = CreateSolidBrush(GetSysColor(COLOR_GRAYTEXT));
        FrameRect(hdc, &panel, gray);
        DeleteObject(gray);

        // Freccia triangolare: fill del colore del pannello + bordo
        // grigio. Il fill cancella il segmento di bordo che cade dentro
        // la base del triangolo (l'apertura verso il target).
        let pts = if arrow_up {
            [
                POINT {
                    x: arrow_ax - ARR_HALF,
                    y: ARROW,
                },
                POINT {
                    x: arrow_ax + ARR_HALF,
                    y: ARROW,
                },
                POINT { x: arrow_ax, y: 0 },
            ]
        } else {
            [
                POINT {
                    x: arrow_ax - ARR_HALF,
                    y: h - ARROW,
                },
                POINT {
                    x: arrow_ax + ARR_HALF,
                    y: h - ARROW,
                },
                POINT {
                    x: arrow_ax,
                    y: h - 1,
                },
            ]
        };
        let pen = CreatePen(PS_SOLID, 1, GetSysColor(COLOR_GRAYTEXT));
        let old_brush = SelectObject(hdc, (COLOR_WINDOW + 1) as HBRUSH);
        let old_pen = SelectObject(hdc, pen);
        Polygon(hdc, pts.as_ptr(), pts.len() as i32);
        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_brush);
        DeleteObject(pen);

        // Glifo di chiusura `✕` nella zona di hit-test.
        if close.right > close.left {
            SetBkMode(hdc, TRANSPARENT as i32);
            SetTextColor(hdc, GetSysColor(COLOR_GRAYTEXT));
            let old_font = SelectObject(hdc, body_font);
            let wide = to_wide("\u{2715}");
            let mut rc_close = close;
            DrawTextW(
                hdc,
                wide.as_ptr(),
                (wide.len() - 1) as i32,
                &mut rc_close,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );
            SelectObject(hdc, old_font);
        }
    }
}

// ── Window procedure ──────────────────────────────────────────────────

#[cfg(target_os = "windows")]
unsafe extern "system" fn teaching_tip_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        // Il pannello è disegnato per intero in `WM_PAINT`.
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            // SAFETY: `BeginPaint`/`EndPaint` accoppiati sullo stesso
            // hwnd; lo stato è opzionale (potrebbe essere in teardown).
            unsafe {
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                let hdc = BeginPaint(hwnd, &mut ps);
                if let Some(rc) = state_of(hwnd) {
                    paint_tip(hwnd, hdc, &rc);
                }
                EndPaint(hwnd, &ps);
            }
            0
        }
        WM_LBUTTONDOWN => {
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            let on_close = {
                let rc = state_of(hwnd);
                match rc {
                    Some(rc) => {
                        let b = rc.borrow();
                        x >= b.close_rect.left
                            && x < b.close_rect.right
                            && y >= b.close_rect.top
                            && y < b.close_rect.bottom
                    }
                    None => false,
                }
            };
            if on_close {
                if let Some(rc) = state_of(hwnd) {
                    hide_tip(&rc);
                }
                0
            } else {
                // Clic fuori dalla ×: la finestra si attiva (così
                // riceve `Esc`) e il default gestisce il resto.
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
        // `Esc` nasconde (solo se il callout è attivo/focalizzato).
        WM_KEYDOWN if wparam == 0x1B => {
            if let Some(rc) = state_of(hwnd) {
                hide_tip(&rc);
            }
            0
        }
        // `WA_INACTIVE` (0): l'utente ha cliccato altrove → nascondi.
        WM_ACTIVATE if (wparam & 0xFFFF) == 0 => {
            if let Some(rc) = state_of(hwnd) {
                hide_tip(&rc);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_DESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let _ = Rc::from_raw(ptr as *const RefCell<TeachingTipInner>);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// ── API pubblica ──────────────────────────────────────────────────────

impl TeachingTip {
    /// Crea un callout con `title` e `body` (non ancora mostrato).
    pub fn new(title: &str, body: &str) -> Self {
        #[cfg(target_os = "windows")]
        let (hwnd, title_hwnd, body_hwnd, title_font, body_font) =
            // SAFETY: `create_tip_window` valida gli handle e li
            // restituisce vivi (la finestra è solo nascosta).
            unsafe { create_tip_window(title, body) };

        let tip = TeachingTip {
            inner: Rc::new(RefCell::new(TeachingTipInner {
                #[cfg(target_os = "windows")]
                hwnd,
                #[cfg(target_os = "windows")]
                title_hwnd,
                #[cfg(target_os = "windows")]
                body_hwnd,
                #[cfg(target_os = "windows")]
                title_font,
                #[cfg(target_os = "windows")]
                body_font,
                #[cfg(target_os = "windows")]
                target: None,
                #[cfg(target_os = "windows")]
                close_rect: RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                },
                #[cfg(target_os = "windows")]
                arrow_up: true,
                #[cfg(target_os = "windows")]
                arrow_ax: 0,
                #[cfg(target_os = "windows")]
                size: (DEFAULT_W, DEFAULT_H),
                #[cfg(not(target_os = "windows"))]
                stub_handle: crate::platform::stub_backend::alloc_widget_handle(),
                title: title.to_string(),
                body: body.to_string(),
                visible: false,
                hiding: false,
                hooked: false,
                on_close: None,
            })),
        };

        // Stato per il wndproc: clone con `Rc::into_raw` (conteggio a
        // 2) dentro `GWLP_USERDATA`.
        #[cfg(target_os = "windows")]
        // SAFETY: `hwnd` è la finestra appena creata, viva e sua.
        unsafe {
            let raw = Rc::into_raw(tip.inner.clone());
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw as isize);
        }

        tip
    }

    /// Mostra il callout agganciato a `target`, seguendo poi `frame`
    /// nei movimenti/redimensionamenti (hook registrati una sola volta).
    pub fn show_for(&self, target: &dyn Widget, frame: &Frame) {
        #[cfg(target_os = "windows")]
        {
            let target_hwnd = target.native_handle() as HWND;
            let (need_hook, rc) = {
                let mut b = self.inner.borrow_mut();
                b.target = Some(target_hwnd);
                let need = !b.hooked;
                b.hooked = true;
                (need, self.inner.clone())
            };
            if need_hook {
                // Segue il frame: riposiziona il callout.
                let rc1 = rc.clone();
                frame.on_move_event(move |_| place(&rc1));
                let rc2 = rc.clone();
                frame.on_resize(move |_, _| place(&rc2));
                // Frame nascosto → callout nascosto.
                frame.on_hide(move |_| hide_tip(&rc));
            }
            place(&self.inner);
            {
                let mut b = self.inner.borrow_mut();
                b.visible = true;
            }
            // SAFETY: handle preso dallo stato, finestra viva.
            unsafe {
                ShowWindow(self.inner.borrow().hwnd, SW_SHOWNOACTIVATE);
                UpdateWindow(self.inner.borrow().hwnd);
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (target, frame);
            self.inner.borrow_mut().visible = true;
        }
    }

    /// Nasconde il callout (senza distruggerlo) e invoca `on_close`.
    pub fn hide(&self) {
        hide_tip(&self.inner);
    }

    /// `true` se il callout è attualmente mostrato.
    pub fn is_visible(&self) -> bool {
        self.inner.borrow().visible
    }

    /// Cambia il titolo (ridimensiona il callout se visibile).
    pub fn set_title(&self, title: &str) {
        self.set_text(true, title);
    }

    /// Cambia il corpo testo (ridimensiona il callout se visibile).
    pub fn set_body(&self, body: &str) {
        self.set_text(false, body);
    }

    /// Registra la callback invocata quando il callout viene nascosto
    /// (da `✕`, `Esc`, perdita di attivazione o [`TeachingTip::hide`]).
    pub fn on_close<F: FnMut() + 'static>(&self, f: F) {
        self.inner.borrow_mut().on_close = Some(Box::new(f));
    }

    /// Dimensione naturale predefinita del callout, per i sizer.
    pub fn default_size() -> (u32, u32) {
        (DEFAULT_W as u32, DEFAULT_H as u32)
    }

    /// Aggiorna titolo (`is_title = true`) o corpo, la `STATIC`
    /// corrispondente e, se visibile, la geometria del callout.
    fn set_text(&self, is_title: bool, text: &str) {
        #[cfg(target_os = "windows")]
        {
            let hwnd = {
                let mut b = self.inner.borrow_mut();
                if is_title {
                    b.title = text.to_string();
                    b.title_hwnd
                } else {
                    b.body = text.to_string();
                    b.body_hwnd
                }
            };
            // SAFETY: `hwnd` è la figlia viva; la stringa wide vive
            // fino a `SetWindowTextW`.
            unsafe {
                let wide = to_wide(text);
                SetWindowTextW(hwnd, wide.as_ptr());
            }
            if self.inner.borrow().visible {
                place(&self.inner);
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let mut b = self.inner.borrow_mut();
            if is_title {
                b.title = text.to_string();
            } else {
                b.body = text.to_string();
            }
        }
    }
}

// ── Drop ──────────────────────────────────────────────────────────────

impl Drop for TeachingTip {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        // SAFETY: handle preso dallo stato; il clone in
        // `GWLP_USERDATA` viene rilasciato una volta sola e la
        // finestra distrutta (come in `busy_info.rs`).
        unsafe {
            let hwnd = self.inner.borrow().hwnd;
            if hwnd.is_null() {
                return;
            }
            // Invalida l'hwnd prima della distruzione: gli hook del
            // frame tengono vivo il `RefCell` e devono diventare no-op.
            self.inner.borrow_mut().hwnd = std::ptr::null_mut();
            self.inner.borrow_mut().visible = false;
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let _ = Rc::from_raw(ptr as *const RefCell<TeachingTipInner>);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            DestroyWindow(hwnd);
        }
    }
}
