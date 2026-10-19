//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! [`TipProvider`] / [`show_tip`] — componente **C8** del piano di
//! portabilità.
//!
//! La finestra tip eroga una sequenza di suggerimenti testuali:
//!
//! * è posizionata **accanto al rettangolo del chiamante** (subito sotto),
//! * applica il **wrap del testo** (`DT_WORDBREAK` su Windows),
//! * offre le **scorciatoie chiave** `INVIO`/spazio → suggerimento
//!   successivo e `ESC` (o un click) → chiudi,
//! * dichiara il ruolo inspector **`dialog`** tramite
//!   `declared::mark`, così `/find?role=dialog` la trova.
//!
//! Su Windows è una popup con classe propria e un **loop modale
//! locale** (modello `Wizard::run`); su macOS una `NSWindow` di dialog
//! con bottoni dotati di key equivalent nativi (`\r` e `ESC`).

use crate::core::widget::Window;

#[cfg(target_os = "macos")]
use objc2::rc::Retained;
#[cfg(target_os = "macos")]
use objc2::{define_class, DefinedClass, MainThreadOnly};
#[cfg(target_os = "macos")]
use objc2_foundation::{MainThreadMarker, NSObjectProtocol};

/// Lunghezza massima (in byte UTF-8) che i provider rispettano quando
/// restituiscono un suggerimento; il testo viene comunque tagliato da
/// [`show_tip`] come difesa (taglio su confine di carattere).
pub const TIP_MAX_LENGTH: usize = 240;

/// Fornitore dei successivi suggerimenti mostrati dalla finestra tip.
pub trait TipProvider {
    /// Restituisce il suggerimento successivo (al massimo
    /// `max_length` byte) oppure `None` quando la serie è esaurita.
    fn next_tip(&mut self, max_length: usize) -> Option<String>;
}

/// Mostra il tip; restituisce true se l'utente ne ha letto almeno uno.
///
/// Il primo suggerimento viene prelevato subito: se il provider è
/// esaurito la funzione ritorna `false` senza aprire alcuna finestra.
/// Altrimenti la finestra resta aperta finché l'utente non chiude
/// (`ESC`, click, `WM_CLOSE`) o la serie non è esaurita
/// (`INVIO`/spazio richiedono il suggerimento successivo).
///
/// ```no_run
/// use ru_wx::{Frame, TipProvider, show_tip};
///
/// struct Tips { items: Vec<String>, idx: usize }
/// impl TipProvider for Tips {
///     fn next_tip(&mut self, max_length: usize) -> Option<String> {
///         let tip = self.items.get(self.idx)?.clone();
///         self.idx += 1;
///         Some(tip.chars().take(max_length).collect())
///     }
/// }
///
/// let frame = Frame::builder().with_title("App").build();
/// let mut tips = Tips { items: vec!["Suggerimento 1".into()], idx: 0 };
/// let read = show_tip(&frame, "Suggerimenti", &mut tips);
/// ```
pub fn show_tip<W: Window>(parent: &W, title: &str, provider: &mut dyn TipProvider) -> bool {
    let mut session = TipSession::default();
    // Nessun suggerimento: niente finestra, niente "letto".
    let Some(first) = session.pull(provider, TIP_MAX_LENGTH) else {
        return false;
    };

    #[cfg(target_os = "windows")]
    {
        run_tip_win32(parent, title, first, provider, &mut session)
    }

    #[cfg(target_os = "macos")]
    {
        run_tip_macos(parent, title, first, provider, &mut session)
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        let _ = (parent, title, provider, first);
        session.read
    }
}

// ── Stato della sessione (piattaforma-indipendente) ─────────────────────

/// Stato della sessione corrente: se almeno un suggerimento è stato
/// effettivamente mostrato (`read`) e da dove proseguire.
#[derive(Default)]
struct TipSession {
    read: bool,
}

impl TipSession {
    /// Preleva il suggerimento successivo, lo ritaglia a
    /// `max_length` e marca la sessione come "letta".
    fn pull(&mut self, provider: &mut dyn TipProvider, max_length: usize) -> Option<String> {
        let tip = provider.next_tip(max_length).map(|t| clip(&t, max_length));
        if tip.is_some() {
            self.read = true;
        }
        tip
    }
}

/// Ritaglia `s` a `max_length` byte rispettando i confini dei
/// caratteri UTF-8 (non taglia mai in mezzo a un code point).
fn clip(s: &str, max_length: usize) -> String {
    if s.len() <= max_length {
        return s.to_string();
    }
    let mut end = max_length;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}

// ── Windows ─────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{SetFocus, VK_ESCAPE, VK_RETURN, VK_SPACE};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Classe window della finestra tip (registrata una volta sola).
#[cfg(target_os = "windows")]
const TIP_PROVIDER_CLASS: &str = "RuWxTipProviderWindowClass";

/// Larghezza fissa della finestra tip (l'altezza dipende dal wrap).
#[cfg(target_os = "windows")]
const TIP_WIDTH: i32 = 360;

/// Margine interno del testo rispetto al bordo.
#[cfg(target_os = "windows")]
const TIP_PAD: i32 = 10;

/// Stato della finestra tip, puntato dal `GWLP_USERDATA` e condiviso
/// tra il loop modale (che preleva i tip dal provider) e il wnd_proc
/// (che traduce le scorciatoie chiave in flag).
#[cfg(target_os = "windows")]
struct TipWndState {
    running: bool,
    wants_next: bool,
    text: Vec<u16>,
}

/// Implementazione Windows: popup con classe propria + loop modale
/// locale. Ritorna `true` se almeno un suggerimento è stato mostrato.
#[cfg(target_os = "windows")]
fn run_tip_win32<W: Window>(
    parent: &W,
    title: &str,
    first: String,
    provider: &mut dyn TipProvider,
    session: &mut TipSession,
) -> bool {
    // SAFETY: chiamate Win32 FFI con argomenti validati; il Box dello
    // stato resta vivo fino a `Box::from_raw` in fondo alla funzione.
    unsafe {
        register_tip_provider_class_once();

        let class = to_wide(TIP_PROVIDER_CLASS);
        let title_w = to_wide(title);
        let state = Box::new(TipWndState {
            running: true,
            wants_next: false,
            text: to_wide(&first),
        });
        let state_ptr = Box::into_raw(state);
        let (mut width, mut height) = measure_tip(&(*state_ptr).text);

        // Posizione: subito sotto il rettangolo del chiamante.
        let anchor = crate::platform::widget_screen_rect(parent.hwnd());
        let x = anchor.x;
        let y = anchor.y + anchor.height as i32 + 4;

        let parent_hwnd = crate::platform::as_hwnd(parent.hwnd());
        let hinstance = GetModuleHandleW(std::ptr::null());
        // `WS_EX_TOOLWINDOW`: niente taskbar/Alt-Tab. `WS_EX_TOPMOST`
        // resta sopra gli altri. A differenza di `TipWindow` la
        // finestra tip DEVE ricevere l'input (scorciatoie chiave), quindi
        // senza `WS_EX_NOACTIVATE`.
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            class.as_ptr(),
            title_w.as_ptr(),
            WS_POPUP | WS_BORDER | WS_VISIBLE,
            x,
            y,
            width,
            height,
            parent_hwnd,
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null_mut(),
        );
        if hwnd.is_null() {
            drop(Box::from_raw(state_ptr));
            return false;
        }
        // Stato leggibile dal wnd_proc.
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
        // Ruolo inspector: la finestra tip è un `dialog` (§6 del piano).
        crate::platform::declared::mark(hwnd as crate::platform::NativeHandle, "dialog");
        ShowWindow(hwnd, SW_SHOW);
        SetFocus(hwnd);

        // Loop modale locale (modello: `Wizard::run`).
        let mut msg: MSG = std::mem::zeroed();
        loop {
            let ret = GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0);
            if ret <= 0 {
                break;
            }
            if !(*state_ptr).running {
                break;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
            if !(*state_ptr).running {
                break;
            }
            // Scorciatoie `INVIO`/spazio: chiedi il suggerimento successivo.
            if (*state_ptr).wants_next {
                (*state_ptr).wants_next = false;
                match session.pull(provider, TIP_MAX_LENGTH) {
                    Some(tip) => {
                        (*state_ptr).text = to_wide(&tip);
                        let (w, h) = measure_tip(&(*state_ptr).text);
                        width = w;
                        height = h;
                        SetWindowPos(
                            hwnd,
                            std::ptr::null_mut(),
                            x,
                            y,
                            width,
                            height,
                            SWP_NOZORDER | SWP_NOACTIVATE,
                        );
                        InvalidateRect(hwnd, std::ptr::null(), 1);
                    }
                    // Serie esaurita: chiudi la finestra.
                    None => (*state_ptr).running = false,
                }
            }
        }

        // Se il loop è uscito senza distruzione (es. `WM_QUIT` da un
        // pump esterno), chiudi comunque la finestra.
        if IsWindow(hwnd) != 0 {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            DestroyWindow(hwnd);
        }
        drop(Box::from_raw(state_ptr));
    }
    session.read
}

/// Registra (una sola volta) la classe window della finestra tip.
#[cfg(target_os = "windows")]
unsafe fn register_tip_provider_class_once() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_name = to_wide(TIP_PROVIDER_CLASS);
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(tip_provider_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: crate::platform::window_icon::class_icons().0,
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: (COLOR_INFOBK + 1) as usize as HBRUSH,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: crate::platform::window_icon::class_icons().1,
        };
        RegisterClassExW(&wc);
    });
}

/// Misura il rettangolo necessario a `text` con wrap a `TIP_WIDTH`
/// (senza margine), restituendo la size esterna della finestra.
#[cfg(target_os = "windows")]
unsafe fn measure_tip(text: &[u16]) -> (i32, i32) {
    let hdc = GetDC(std::ptr::null_mut());
    if hdc.is_null() {
        return (TIP_WIDTH, 60);
    }
    let mut rc: RECT = RECT {
        left: 0,
        top: 0,
        right: TIP_WIDTH - 2 * TIP_PAD,
        bottom: 0,
    };
    if !text.is_empty() {
        DrawTextW(
            hdc,
            text.as_ptr(),
            text.len() as i32,
            &mut rc,
            DT_CALCRECT | DT_WORDBREAK,
        );
    }
    ReleaseDC(std::ptr::null_mut(), hdc);
    let mut height = rc.bottom - rc.top + 2 * TIP_PAD;
    if height < 48 {
        height = 48;
    }
    (TIP_WIDTH, height)
}

/// Window procedure della finestra tip: disegna il testo con wrap e
/// traduce le scorciatoie chiave in flag per il loop modale.
///
/// * `INVIO` / spazio → `wants_next` (suggerimento successivo),
/// * `ESC` / `WM_CLOSE` / click → `running = false` (chiudi).
#[cfg(target_os = "windows")]
unsafe extern "system" fn tip_provider_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut TipWndState;
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            if !state.is_null() {
                let mut rc: RECT = std::mem::zeroed();
                GetClientRect(hwnd, &mut rc);
                rc.left += TIP_PAD;
                rc.top += TIP_PAD;
                rc.right -= TIP_PAD;
                rc.bottom -= TIP_PAD;
                let text = &(*state).text;
                if !text.is_empty() && rc.right > rc.left {
                    // Wrap del testo sullo spazio disponibile.
                    DrawTextW(hdc, text.as_ptr(), text.len() as i32, &mut rc, DT_WORDBREAK);
                }
            }
            EndPaint(hwnd, &ps);
            return 0;
        }
        WM_KEYDOWN => {
            if state.is_null() {
                return 0;
            }
            if wparam as u32 == VK_ESCAPE as u32 {
                (*state).running = false;
                DestroyWindow(hwnd);
                return 0;
            }
            if wparam as u32 == VK_RETURN as u32 || wparam as u32 == VK_SPACE as u32 {
                (*state).wants_next = true;
                return 0;
            }
        }
        WM_LBUTTONDOWN | WM_RBUTTONDOWN => {
            // Come `TipWindow`: un click fa avanzare la serie.
            if !state.is_null() {
                (*state).wants_next = true;
                return 0;
            }
        }
        WM_CLOSE | WM_DESTROY if !state.is_null() => {
            (*state).running = false;
        }
        _ => {}
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

// ── macOS ───────────────────────────────────────────────────────────────

/// Flag di sessione del loop modale mac (una finestra tip alla volta).
#[cfg(target_os = "macos")]
thread_local! {
    static TIP_NEXT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static TIP_RUNNING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Target ObjC dei due bottoni della finestra tip mac: `kind = 0`
/// richiede il suggerimento successivo, `kind = 1` chiude.
#[cfg(target_os = "macos")]
#[derive(Default)]
struct TipActionIvars {
    kind: std::cell::Cell<u8>,
}

#[cfg(target_os = "macos")]
define_class!(
    #[unsafe(super = objc2_foundation::NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = TipActionIvars]
    struct TipActionTarget;

    unsafe impl NSObjectProtocol for TipActionTarget {}

    impl TipActionTarget {
        #[unsafe(method(ruWxTipAction:))]
        fn tip_action(&self, _sender: Option<&objc2::runtime::AnyObject>) {
            if self.ivars().kind.get() == 0 {
                TIP_NEXT.with(|c| c.set(true));
            } else {
                TIP_RUNNING.with(|c| c.set(false));
            }
        }
    }
);

#[cfg(target_os = "macos")]
impl TipActionTarget {
    /// Crea una nuova istanza con il tipo specificato.
    fn new(mtm: MainThreadMarker, kind: u8) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(TipActionIvars {
            kind: std::cell::Cell::new(kind),
        });
        // SAFETY: inizializzazione dell'oggetto NSObject appena allocato.
        unsafe { objc2::msg_send![super(this), init] }
    }
}

/// Implementazione macOS: `NSWindow` di dialog (modello
/// `single_choice_dialog`) con label + bottoni con key equivalent.
#[cfg(target_os = "macos")]
fn run_tip_macos<W: Window>(
    parent: &W,
    title: &str,
    first: String,
    provider: &mut dyn TipProvider,
    session: &mut TipSession,
) -> bool {
    use objc2_app_kit::{NSBezelStyle, NSButton};
    use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};

    use crate::core::geometry::Rect;
    use crate::platform::appkit::{
        create_dialog_window, create_static_text, dialog_close, dialog_content_view,
        dialog_run_until, set_static_text,
    };
    use crate::platform::{declared, window_handle};

    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let dlg = create_dialog_window(window_handle(parent), title, 460, 280);
    // Ruolo inspector: la finestra tip è un `dialog` (§6 del piano).
    declared::mark(dlg, "dialog");
    let label = create_static_text(dlg, &first, Rect::new(16, 84, 428, 170));

    TIP_NEXT.with(|c| c.set(false));
    TIP_RUNNING.with(|c| c.set(true));

    // Bottoni con scorciatoie chiave native: `INVIO` → prossimo tip,
    // `ESC` → chiudi.
    let content = dialog_content_view(dlg).expect("content view della dialog tip");

    let next_btn = NSButton::initWithFrame(
        NSButton::alloc(mtm),
        NSRect::new(NSPoint::new(248.0, 16.0), NSSize::new(96.0, 28.0)),
    );
    next_btn.setTitle(&NSString::from_str("Next tip"));
    next_btn.setBezelStyle(NSBezelStyle::Rounded);
    next_btn.setKeyEquivalent(&NSString::from_str("\r"));
    let next_target = TipActionTarget::new(mtm, 0);
    // SAFETY: target/action classici di NSButton.
    unsafe {
        next_btn.setTarget(Some(&next_target));
        next_btn.setAction(Some(objc2::sel!(ruWxTipAction:)));
    }
    content.addSubview(&next_btn);

    let close_btn = NSButton::initWithFrame(
        NSButton::alloc(mtm),
        NSRect::new(NSPoint::new(352.0, 16.0), NSSize::new(92.0, 28.0)),
    );
    close_btn.setTitle(&NSString::from_str("Close"));
    close_btn.setBezelStyle(NSBezelStyle::Rounded);
    close_btn.setKeyEquivalent(&NSString::from_str("\u{1b}"));
    let close_target = TipActionTarget::new(mtm, 1);
    // SAFETY: target/action classici di NSButton.
    unsafe {
        close_btn.setTarget(Some(&close_target));
        close_btn.setAction(Some(objc2::sel!(ruWxTipAction:)));
    }
    content.addSubview(&close_btn);

    // Loop modale locale: mentre la finestra è aperta, processa le
    // richieste "prossimo tip" emesse dai bottoni.
    dialog_run_until(dlg, || {
        if TIP_NEXT.with(|c| c.replace(false)) {
            match session.pull(provider, TIP_MAX_LENGTH) {
                Some(tip) => set_static_text(label, &tip),
                // Serie esaurita: chiudi.
                None => TIP_RUNNING.with(|c| c.set(false)),
            }
        }
        TIP_RUNNING.with(|c| c.get())
    });
    dialog_close(dlg);
    session.read
}

// ── Test ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Provider finto che eroga una lista fissa di suggerimenti.
    struct VecProvider {
        tips: Vec<String>,
        idx: usize,
    }

    impl TipProvider for VecProvider {
        fn next_tip(&mut self, _max_length: usize) -> Option<String> {
            let tip = self.tips.get(self.idx)?.clone();
            self.idx += 1;
            Some(tip)
        }
    }

    #[test]
    fn clip_taglia_su_confine_utf8() {
        assert_eq!(clip("abcdefg", 3), "abc");
        assert_eq!(clip("abc", 3), "abc");
        // 3 byte: il 3° cade dentro β, si arretra al confine → "α" (2 byte).
        assert_eq!(clip("αβγδ", 3), "α");
        assert_eq!(clip("αβγδ", 4), "αβ");
        assert_eq!(clip("αβγδ", 5), "αβ");
        assert_eq!(clip("", 5), "");
    }

    #[test]
    fn session_pull_ritaglia_e_marca_letto() {
        let mut provider = VecProvider {
            tips: vec!["first tip".into(), "second".into()],
            idx: 0,
        };
        let mut session = TipSession::default();
        assert!(!session.read);

        assert_eq!(session.pull(&mut provider, 4).as_deref(), Some("firs"));
        assert!(session.read);
        assert_eq!(session.pull(&mut provider, 4).as_deref(), Some("seco"));

        // Serie esaurita.
        assert_eq!(session.pull(&mut provider, 4), None);
        assert!(session.read);
    }

    #[test]
    fn session_con_provider_vuoto_non_leta() {
        let mut provider = VecProvider {
            tips: Vec::new(),
            idx: 0,
        };
        let mut session = TipSession::default();
        assert_eq!(session.pull(&mut provider, TIP_MAX_LENGTH), None);
        assert!(!session.read);
    }
}
