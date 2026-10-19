//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! BreadcrumbBar — barra di navigazione a briciole di pane.
//!
//! # Nota architetturale (scelta API)
//!
//! La classe nativa `msctls_breadcrumb32` con le API `BREADCRUMBITEM`,
//! `BCM_SETITEMS` e `BCN_GETITEMS` **non è esposta da windows-sys 0.59**
//! (verificato per grep sul registry della dipendenza): la variante nativa
//! richiede inoltre item PIDL, molto vincolante per un percorso di testo.
//! Come previsto dalla specifica, la navigazione è quindi implementata con
//! **item testuali tramite la API più semplice disponibile**:
//!
//! - un contenitore figlio (classe `STATIC` predefinita, senza
//!   `RegisterClassExW`) ospita un pulsante per segmento del percorso;
//! - il contenitore viene sub-classato (stesso pattern di
//!   [`crate::controls::list_ctrl`]) per intercettare `WM_COMMAND` +
//!   `BN_CLICKED` generato dai pulsanti figli e smistarlo alla callback
//!   registrata con [`BreadcrumbBar::on_navigate`];
//! - nessun `SetWindowTheme`: i controlli usano il tema di sistema (Win11);
//! - le classi `STATIC`/`BUTTON` rientrano in `ICC_STANDARD_CLASSES`, già
//!   presente in `InitCommonControlsEx` (`core::app.rs`): nessun `ICC_*` nuovo.
//!
//! # Esempio
//!
//! ```no_run
//! use ru_wx::controls::breadcrumb_bar::BreadcrumbBar;
//! use ru_wx::{App, Frame};
//!
//! let app = App::new();
//! let frame = Frame::builder().with_title("Demo").with_size(800, 600).build();
//! let bar = BreadcrumbBar::new(&frame);
//! bar.set_path(r"C:\Program Files\ru_wx\docs");
//! bar.on_navigate(&frame, |idx, path| {
//!     println!("segmento {idx}: {path}");
//! });
//! # let _ = app;
//! ```

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::geometry::Rect;
use crate::core::tokens::{CONTROL_HEIGHT, SPACING_M, SPACING_S, SPACING_XS};
use crate::core::widget::{Widget, WidgetRef, Window};
use crate::window::frame::Frame;

use crate::platform::next_control_id;
#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{
    GetDC, GetStockObject, GetTextExtentPoint32W, ReleaseDC, SelectObject, DEFAULT_GUI_FONT, HDC,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::SystemServices::{SS_CENTER, SS_LEFT};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Separatore testuale tra due segmenti (glifo ASCII, sempre disponibile
/// nel font di sistema).
#[cfg(target_os = "windows")]
const SEGMENT_SEPARATOR: &str = ">";

struct BreadcrumbBarInner {
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    rect: Rect,
    visible: bool,
    enabled: bool,
    /// Segmenti attualmente visualizzati (stato interno della barra).
    items: Vec<String>,
    /// Percorso corrente esposto da [`BreadcrumbBar::current_path`].
    path: String,
    /// Id dei pulsanti-segmento, nell'ordine di `items` (routing del click).
    #[cfg(target_os = "windows")]
    segment_ids: Vec<u16>,
    /// Handle dei pulsanti-segmento (propagazione di `set_enabled`).
    #[cfg(target_os = "windows")]
    segment_hwnds: Vec<HWND>,
    /// Handle dei separatori `>`, in numero pari a `items - 1`.
    #[cfg(target_os = "windows")]
    separator_hwnds: Vec<HWND>,
    /// Callback di navigazione registrata con [`BreadcrumbBar::on_navigate`].
    handler: Option<Box<dyn FnMut(usize, String)>>,
}

#[derive(Clone)]
pub struct BreadcrumbBar {
    inner: Rc<RefCell<BreadcrumbBarInner>>,
}

impl BreadcrumbBar {
    /// Crea una breadcrumb bar vuota come figlia di `parent`.
    pub fn new<W: Window>(parent: &W) -> Self {
        let id = next_control_id();

        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        let hwnd = unsafe {
            let parent_hwnd = crate::platform::window_handle_as_hwnd(parent);
            let wide_class = to_wide("STATIC");
            CreateWindowExW(
                0,
                wide_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | SS_LEFT,
                0,
                0,
                400,
                CONTROL_HEIGHT,
                parent_hwnd,
                id as usize as HMENU,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        #[cfg(not(target_os = "windows"))]
        let _ = (parent, id);

        let inner = Rc::new(RefCell::new(BreadcrumbBarInner {
            #[cfg(target_os = "windows")]
            hwnd,
            rect: Rect::new(0, 0, 400, CONTROL_HEIGHT as u32),
            visible: true,
            enabled: true,
            items: Vec::new(),
            path: String::new(),
            #[cfg(target_os = "windows")]
            segment_ids: Vec::new(),
            #[cfg(target_os = "windows")]
            segment_hwnds: Vec::new(),
            #[cfg(target_os = "windows")]
            separator_hwnds: Vec::new(),
            handler: None,
        }));

        // Sub-classa il contenitore per intercettare `WM_COMMAND` dei figli.
        #[cfg(target_os = "windows")]
        install_breadcrumb_subclass(hwnd, inner.clone());

        BreadcrumbBar { inner }
    }

    /// Sostituisce gli item testuali visualizzati; il percorso corrente
    /// diventa l'insieme degli item uniti da `\`.
    pub fn set_items(&self, items: &[&str]) {
        let items: Vec<String> = items.iter().map(|s| (*s).to_string()).collect();
        let mut inner = self.inner.borrow_mut();
        inner.path = items.join("\\");
        inner.items = items;
        #[cfg(target_os = "windows")]
        inner.rebuild();
    }

    /// Imposta il percorso corrente: i segmenti vengono ricavati separando
    /// su `\` e `/` (i separatori vuoti vengono scartati).
    pub fn set_path(&self, path: &str) {
        let items: Vec<String> = path
            .split(['\\', '/'])
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        let mut inner = self.inner.borrow_mut();
        inner.path = path.to_string();
        inner.items = items;
        #[cfg(target_os = "windows")]
        inner.rebuild();
    }

    /// Percorso corrente impostato con [`set_path`](Self::set_path) /
    /// [`set_items`](Self::set_items).
    pub fn current_path(&self) -> String {
        self.inner.borrow().path.clone()
    }

    /// Registra la callback di navigazione: `f(indice, percorso)`, dove il
    /// percorso è la radice dei segmenti da 0 fino a quello cliccato.
    ///
    /// Il parametro `frame` è richiesto dalla firma dell'API (coerenza con
    /// `SearchCtrl::on_change`): il routing del click è gestito dal
    /// sub-classing del contenitore, quindi nessun handler va registrato
    /// sul frame.
    pub fn on_navigate<F: FnMut(usize, String) + 'static>(&self, _frame: &Frame, f: F) {
        self.inner.borrow_mut().handler = Some(Box::new(f));
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.inner.clone()
    }
}

impl BreadcrumbBarInner {
    /// Ricostruisce i pulsanti-segmento da `self.items`.
    #[cfg(target_os = "windows")]
    fn rebuild(&mut self) {
        // SAFETY: i figli sono finestre che abbiamo creato con `CreateWindowExW`;
        // si raccolgono i puntatori al prossimo prima di ogni `DestroyWindow`.
        unsafe {
            let mut child = GetWindow(self.hwnd, GW_CHILD);
            while !child.is_null() {
                let next = GetWindow(child, GW_HWNDNEXT);
                DestroyWindow(child);
                child = next;
            }
        }
        self.segment_ids.clear();
        self.segment_hwnds.clear();
        self.separator_hwnds.clear();

        let items = std::mem::take(&mut self.items);
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            let hfont = GetStockObject(DEFAULT_GUI_FONT);
            let wide_btn = to_wide("BUTTON");
            let wide_static = to_wide("STATIC");
            let wide_sep = to_wide(SEGMENT_SEPARATOR);
            for (i, item) in items.iter().enumerate() {
                let seg_id = next_control_id();
                let wide_item = to_wide(item);
                let hwnd_btn = CreateWindowExW(
                    0,
                    wide_btn.as_ptr(),
                    wide_item.as_ptr(),
                    WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON as u32,
                    0,
                    0,
                    0,
                    0,
                    self.hwnd,
                    seg_id as usize as HMENU,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                SendMessageW(hwnd_btn, WM_SETFONT, hfont as usize, 1);
                self.segment_ids.push(seg_id);
                self.segment_hwnds.push(hwnd_btn);

                if i + 1 < items.len() {
                    let hwnd_sep = CreateWindowExW(
                        0,
                        wide_static.as_ptr(),
                        wide_sep.as_ptr(),
                        WS_CHILD | WS_VISIBLE | SS_CENTER,
                        0,
                        0,
                        0,
                        0,
                        self.hwnd,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    );
                    SendMessageW(hwnd_sep, WM_SETFONT, hfont as usize, 1);
                    self.separator_hwnds.push(hwnd_sep);
                }
            }
        }
        self.items = items;
        self.layout_segments();
    }

    /// Riposiziona i figli (pulsanti + separatori) nel contenitore,
    /// misurando il testo con il font di sistema.
    #[cfg(target_os = "windows")]
    fn layout_segments(&mut self) {
        // SAFETY: `GetDC`/`ReleaseDC` su un contenitore vivo; il DC viene
        // rilasciato in ogni percorso e il vecchio font ripristinato.
        unsafe {
            let hdc = GetDC(self.hwnd);
            if hdc.is_null() {
                return;
            }
            let hfont = GetStockObject(DEFAULT_GUI_FONT);
            let old_font = SelectObject(hdc, hfont);
            let seg_y = 2;
            let seg_h = (self.rect.height as i32 - 2 * seg_y).max(1);
            let mut x = SPACING_S;
            for (i, item) in self.items.iter().enumerate() {
                let w = measure_text_width(hdc, item) + 2 * SPACING_M;
                MoveWindow(self.segment_hwnds[i], x, seg_y, w, seg_h, 1);
                x += w;
                if let Some(&hwnd_sep) = self.separator_hwnds.get(i) {
                    let sep_w = measure_text_width(hdc, SEGMENT_SEPARATOR) + SPACING_XS;
                    MoveWindow(hwnd_sep, x, seg_y, sep_w, seg_h, 1);
                    x += sep_w + SPACING_XS;
                }
            }
            SelectObject(hdc, old_font);
            ReleaseDC(self.hwnd, hdc);
        }
    }

    /// Propaga l'abilitazione al contenitore e ai pulsanti figli
    /// (`EnableWindow` su una finestra non la estende ai figli).
    #[cfg(target_os = "windows")]
    fn apply_enabled(&self, enabled: bool) {
        let flag = if enabled { 1 } else { 0 };
        // SAFETY: handle validi creati dalla barra.
        unsafe {
            EnableWindow(self.hwnd, flag);
            for &hwnd in &self.segment_hwnds {
                EnableWindow(hwnd, flag);
            }
        }
    }
}

/// Larghezza in pixel di `text` con il font corrente del DC.
#[cfg(target_os = "windows")]
fn measure_text_width(hdc: HDC, text: &str) -> i32 {
    let wide = to_wide(text);
    let mut size = windows_sys::Win32::Foundation::SIZE { cx: 0, cy: 0 };
    // SAFETY: `wide` è vivo per tutta la chiamata e `size` ha la dimensione
    // richiesta da `GetTextExtentPoint32W`.
    unsafe {
        let _ = GetTextExtentPoint32W(hdc, wide.as_ptr(), (wide.len() - 1) as i32, &mut size);
    }
    size.cx
}

// ── Subclass Win32 (routing WM_COMMAND dei segmenti) ─────────────────

/// `WndProc` originale di un contenitore subclassato.
#[cfg(target_os = "windows")]
type BreadcrumbOrigProc = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

#[cfg(target_os = "windows")]
thread_local! {
    /// `WndProc` originale salvato per ogni HWND subclassato.
    static BREADCRUMB_ORIG_PROCS: RefCell<std::collections::HashMap<isize, BreadcrumbOrigProc>> =
        RefCell::new(std::collections::HashMap::new());
    /// Back-reference HWND → stato della barra, così il subclass può
    /// tradurre l'id del pulsante premuto in indice + percorso.
    static BREADCRUMB_SUBCLASS_INNERS: RefCell<
        std::collections::HashMap<isize, Rc<RefCell<BreadcrumbBarInner>>>,
    > = RefCell::new(std::collections::HashMap::new());
}

/// Subclassa il contenitore per intercettare le notifiche `WM_COMMAND`
/// dei pulsanti-segmento figli.
#[cfg(target_os = "windows")]
fn install_breadcrumb_subclass(hwnd: HWND, inner: Rc<RefCell<BreadcrumbBarInner>>) {
    if hwnd.is_null() {
        return;
    }
    // SAFETY: `hwnd` è il contenitore vivo creato da `CreateWindowExW`;
    // salviamo il WndProc originale e ne installiamo uno nostro.
    unsafe {
        let key = hwnd as isize;
        BREADCRUMB_SUBCLASS_INNERS.with(|m| m.borrow_mut().insert(key, inner));
        let original = GetWindowLongPtrW(hwnd, GWLP_WNDPROC) as usize;
        let original_proc: BreadcrumbOrigProc = std::mem::transmute(original);
        BREADCRUMB_ORIG_PROCS.with(|m| m.borrow_mut().insert(key, original_proc));
        SetWindowLongPtrW(
            hwnd,
            GWLP_WNDPROC,
            breadcrumb_wnd_proc as *const () as usize as isize,
        );
    }
}

/// `WndProc` del subclass: su pressione di un pulsante-segmento invoca la
/// callback di navigazione con `(indice, percorso)`; poi inoltra tutto al
/// `WndProc` originale.
#[cfg(target_os = "windows")]
unsafe extern "system" fn breadcrumb_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_COMMAND {
        let notify_code = ((wparam >> 16) & 0xFFFF) as u32;
        let ctrl_id = (wparam & 0xFFFF) as u16;
        if notify_code == BN_CLICKED {
            let inner =
                BREADCRUMB_SUBCLASS_INNERS.with(|m| m.borrow().get(&(hwnd as isize)).cloned());
            if let Some(inner) = inner {
                let target = {
                    let b = inner.borrow();
                    b.segment_ids
                        .iter()
                        .position(|&id| id == ctrl_id)
                        .map(|idx| (idx, b.items[..=idx].join("\\")))
                };
                if let Some((idx, path)) = target {
                    // La callback viene staccata prima dell'invocazione così
                    // può richiamare `set_items`/`set_path` senza double-borrow.
                    let mut handler = inner.borrow_mut().handler.take();
                    if let Some(handler) = handler.as_mut() {
                        handler(idx, path);
                    }
                    inner.borrow_mut().handler = handler;
                }
            }
        }
    }

    let orig = BREADCRUMB_ORIG_PROCS.with(|m| m.borrow().get(&(hwnd as isize)).copied());
    if msg == WM_NCDESTROY {
        BREADCRUMB_SUBCLASS_INNERS.with(|m| m.borrow_mut().remove(&(hwnd as isize)));
        BREADCRUMB_ORIG_PROCS.with(|m| m.borrow_mut().remove(&(hwnd as isize)));
    }
    if let Some(orig) = orig {
        orig(hwnd, msg, wparam, lparam)
    } else {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

impl Widget for BreadcrumbBarInner {
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
            self.layout_segments();
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
        #[cfg(target_os = "windows")]
        self.apply_enabled(enabled);
    }
}
