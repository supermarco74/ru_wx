//! Modern Window API - Porting da windows-rs
//!
//! Questo modulo implementa un pattern moderno per la creazione e gestione
//! di finestre Win32, basato sullo stile di windows-rs:
//!
//! - `WindowBuilder` con chaining dei builder method
//! - `run()`, `run_with()`, `pump()` per il message loop
//! - `MessageHandler` per intercettare messaggi raw
//! - `Result<()>` per error handling
//!
//! # Esempio
//!
//! ```ignore
//! use ru_wx::platform::winui3::window::{Window, WindowBuilder, run, run_with};
//!
//! // Creazione con builder pattern
//! let window = WindowBuilder::new("Titolo Finestra")
//!     .size(800, 600)
//!     .position(100, 100)
//!     .style(WS_OVERLAPPEDWINDOW)
//!     .on_message(|hwnd, msg, w, l| {
//!         if msg == WM_CLOSE {
//!             Some(0) // handled
//!         } else {
//!             None     // default
//!         }
//!     })
//!     .on_resize(|w, h| {
//!         println!("Resize: {}x{}", w, h);
//!     })
//!     .create()
//!     .unwrap();
//!
//! // Message loop blocking
//! run();
//! ```

use std::sync::OnceLock;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ============================================================================
// Error Handling
// ============================================================================

/// Risultato di operazioni che possono fallire con messaggio di errore Win32.
pub type Result<T> = std::result::Result<T, Win32Error>;

/// Errore Win32 con messaggio descrittivo.
#[derive(Debug, Clone)]
pub struct Win32Error {
    pub code: u32,
    pub message: String,
}

impl Win32Error {
    /// Crea un errore dal codice di sistema corrente.
    pub fn from_last_error() -> Self {
        unsafe {
            let code = GetLastError();
            Self::from_code(code)
        }
    }

    /// Crea un errore da un codice specifico.
    pub fn from_code(code: u32) -> Self {
        let message = Self::format_message(code);
        Self { code, message }
    }

    fn format_message(code: u32) -> String {
        // Formattiamo un messaggio base senza dipendere da FormatMessageW
        format!("Win32 error code: {:#x}", code)
    }
}

impl std::fmt::Display for Win32Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Win32Error({:#x}): {}", self.code, self.message)
    }
}

impl std::error::Error for Win32Error {}

// ============================================================================
// Message Types
// ============================================================================

/// Costanti WM_* comuni per uso nel message handler.
pub mod wm {
    pub const CLOSE: u32 = 0x0010;
    pub const DESTROY: u32 = 0x0002;
    pub const QUIT: u32 = 0x0012;
    pub const SIZE: u32 = 0x0005;
    pub const PAINT: u32 = 0x000F;
    pub const KEYDOWN: u32 = 0x0100;
    pub const KEYUP: u32 = 0x0101;
    pub const CHAR: u32 = 0x0102;
    pub const MOUSEWHEEL: u32 = 0x020A;
    pub const LBUTTONDOWN: u32 = 0x0201;
    pub const RBUTTONDOWN: u32 = 0x0204;
    pub const MBUTTONDOWN: u32 = 0x0207;
    pub const LBUTTONUP: u32 = 0x0202;
    pub const RBUTTONUP: u32 = 0x0205;
    pub const MBUTTONUP: u32 = 0x0208;
    pub const MOUSEMOVE: u32 = 0x0200;
    pub const NCDESTROY: u32 = 0x0082;
}

/// Costanti comuni per stili finestra.
pub mod ws {
    pub const OVERLAPPEDWINDOW: u32 = 0x00CF0000;
    pub const POPUPWINDOW: u32 = 0x80880000;
    pub const CHILD: u32 = 0x40000000;
    pub const VISIBLE: u32 = 0x10000000;
    pub const DISABLED: u32 = 0x08000000;
    pub const CAPTION: u32 = 0x00C00000;
    pub const SYSMENU: u32 = 0x00080000;
    pub const THICKFRAME: u32 = 0x00040000;
    pub const MINIMIZEBOX: u32 = 0x00020000;
    pub const MAXIMIZEBOX: u32 = 0x00010000;
}

// ============================================================================
// State & Handler Types
// ============================================================================

/// Handler per messaggi grezzi della finestra.
/// Riceve (hwnd, message, wparam, lparam).
/// Ritorna `Some(result)` se il messaggio è stato gestito,
/// `None` per usare il processing di default.
pub type MessageHandler = Box<dyn FnMut(HWND, u32, usize, isize) -> Option<isize>>;

/// Handler per resize della finestra.
/// Riceve (width, height) della client area.
pub type ResizeHandler = Box<dyn FnMut(i32, i32)>;

/// Handler per paint (WM_PAINT).
pub type PaintHandler = Box<dyn FnMut() -> bool>;

/// Stato interno della finestra.
struct State {
    message: Option<MessageHandler>,
    resize: Option<ResizeHandler>,
    paint: Option<PaintHandler>,
}

// ============================================================================
// Window
// ============================================================================

/// Finestra top-level Win32.
///
/// La finestra esiste finché non viene chiusa o droppata.
/// L'HWND grezzo è disponibile via `Window::hwnd()`.
pub struct Window(HWND);

impl Window {
    /// Crea un nuovo WindowBuilder con il titolo specificato.
    #[allow(clippy::new_ret_no_self)] // intentional builder pattern
    pub fn new(title: &str) -> WindowBuilder {
        WindowBuilder::new(title)
    }

    /// Ritorna l'HWND grezzo per interoperabilità con altre API Windows.
    pub fn hwnd(&self) -> HWND {
        self.0
    }

    /// Ritorna la dimensione corrente della client area come `(width, height)`.
    pub fn client_size(&self) -> (i32, i32) {
        unsafe {
            let mut rect = std::mem::zeroed::<RECT>();
            if GetClientRect(self.0, &mut rect) != 0 {
                (rect.right - rect.left, rect.bottom - rect.top)
            } else {
                (0, 0)
            }
        }
    }

    /// Ritorna la dimensione corrente della finestra (includendo bordi) come `(width, height)`.
    pub fn window_size(&self) -> (i32, i32) {
        unsafe {
            let mut rect = std::mem::zeroed::<RECT>();
            if GetWindowRect(self.0, &mut rect) != 0 {
                (rect.right - rect.left, rect.bottom - rect.top)
            } else {
                (0, 0)
            }
        }
    }

    /// Centra la finestra sullo schermo.
    pub fn center(&self) {
        unsafe {
            let mut rect = std::mem::zeroed::<RECT>();
            let mut screen = std::mem::zeroed::<RECT>();
            GetWindowRect(self.0, &mut rect);
            SystemParametersInfoW(0x0030, 0, &mut screen as *mut _ as *mut _, 0); // SPI_GETWORKAREA
            let x = (screen.right - screen.left - (rect.right - rect.left)) / 2;
            let y = (screen.bottom - screen.top - (rect.bottom - rect.top)) / 2;
            SetWindowPos(self.0, std::ptr::null_mut(), x, y, 0, 0, 0x0004 | 0x0001);
            // SWP_NOSIZE | SWP_NOZORDER
        }
    }

    /// Mostra o nasconde la finestra.
    pub fn show(&self, visible: bool) {
        unsafe {
            ShowWindow(self.0, if visible { SW_SHOW } else { SW_HIDE });
        }
    }

    /// Ridimensiona la finestra.
    pub fn resize(&self, width: i32, height: i32) {
        unsafe {
            SetWindowPos(
                self.0,
                std::ptr::null_mut(),
                0,
                0,
                width,
                height,
                0x0004 | 0x0002,
            ); // SWP_NOMOVE | SWP_NOZORDER
        }
    }

    /// Muove la finestra alla posizione specificata.
    pub fn move_to(&self, x: i32, y: i32) {
        unsafe {
            SetWindowPos(self.0, std::ptr::null_mut(), x, y, 0, 0, 0x0004 | 0x0001);
            // SWP_NOSIZE | SWP_NOZORDER
        }
    }

    /// Imposta il titolo della finestra.
    pub fn set_title(&self, title: &str) {
        unsafe {
            let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
            SetWindowTextW(self.0, wide.as_ptr());
        }
    }

    /// Invalida l'intera client area, causando WM_PAINT.
    pub fn invalidate(&self) {
        unsafe {
            InvalidateRect(self.0, std::ptr::null(), 1); // erase background
        }
    }

    /// Invalida un'area specifica.
    pub fn invalidate_rect(&self, x: i32, y: i32, w: i32, h: i32) {
        unsafe {
            let rect = RECT {
                left: x,
                top: y,
                right: x + w,
                bottom: y + h,
            };
            InvalidateRect(self.0, &rect, 1);
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            if IsWindow(self.0) != 0 {
                let _ = DestroyWindow(self.0);
            }
        }
    }
}

// Implement WidgetParent so Window can be used as parent for other controls
impl super::widgets::WidgetParent for Window {
    fn hwnd(&self) -> HWND {
        self.0
    }
}

// ============================================================================
// WindowBuilder
// ============================================================================

/// Builder per configurare e creare una Window.
pub struct WindowBuilder {
    title: String,
    width: i32,
    height: i32,
    x: i32,
    y: i32,
    style: u32,
    ex_style: u32,
    state: State,
}

impl WindowBuilder {
    /// Crea un nuovo builder con il titolo specificato.
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            width: CW_USEDEFAULT,
            height: CW_USEDEFAULT,
            x: CW_USEDEFAULT,
            y: CW_USEDEFAULT,
            style: WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            ex_style: 0,
            state: State {
                message: None,
                resize: None,
                paint: None,
            },
        }
    }

    /// Imposta la dimensione iniziale in pixel.
    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Imposta la posizione iniziale in pixel (screen coordinates).
    pub fn position(mut self, x: i32, y: i32) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    /// Imposta lo stile della finestra (es. WS_OVERLAPPEDWINDOW).
    pub fn style(mut self, style: u32) -> Self {
        self.style = style;
        self
    }

    /// Aggiunge stili finestra (OR bitwise).
    pub fn with_style(mut self, style: u32) -> Self {
        self.style |= style;
        self
    }

    /// Rimuove stili finestra (AND NOT bitwise).
    pub fn without_style(mut self, style: u32) -> Self {
        self.style &= !style;
        self
    }

    /// Imposta gli stili estesi (WS_EX_*).
    pub fn ex_style(mut self, ex_style: u32) -> Self {
        self.ex_style = ex_style;
        self
    }

    /// Imposta un handler per ogni messaggio della finestra.
    /// Ritorna `Some(result)` per gestire il messaggio, `None` per
    /// il processing di default.
    pub fn on_message<F>(mut self, handler: F) -> Self
    where
        F: FnMut(HWND, u32, usize, isize) -> Option<isize> + 'static,
    {
        self.state.message = Some(Box::new(handler));
        self
    }

    /// Imposta un handler chiamato quando la client area viene ridimensionata.
    /// Riceve (width, height) in pixel.
    pub fn on_resize<F>(mut self, handler: F) -> Self
    where
        F: FnMut(i32, i32) + 'static,
    {
        self.state.resize = Some(Box::new(handler));
        self
    }

    /// Imposta un handler chiamato quando la finestra richiede painting.
    /// Ritorna `true` per continuare a ridisegnare, `false` per fermare.
    pub fn on_paint<F>(mut self, handler: F) -> Self
    where
        F: FnMut() -> bool + 'static,
    {
        self.state.paint = Some(Box::new(handler));
        self
    }

    /// Crea e mostra la finestra.
    pub fn create(self) -> Result<Window> {
        unsafe {
            register_window_class();

            let mut title: Vec<u16> = self.title.encode_utf16().collect();
            title.push(0);

            let hwnd = CreateWindowExW(
                self.ex_style,
                class_name(),
                title.as_ptr(),
                self.style,
                self.x,
                self.y,
                self.width,
                self.height,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            );

            if hwnd.is_null() {
                return Err(Win32Error::from_last_error());
            }

            // Store state in GWLP_USERDATA
            let state = Box::new(self.state);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);

            Ok(Window(hwnd))
        }
    }

    /// Crea la finestra e ritorna anche il builder per chaining.
    /// (Alias per `create()` per compatibilità con idiomi esistenti)
    pub fn build(self) -> Result<Window> {
        self.create()
    }
}

// ============================================================================
// Message Loop Functions
// ============================================================================

/// Esegue un message loop bloccante fino alla chiusura della finestra.
/// Questa è la forma più semplice - non chiama callback di render.
pub fn run() {
    unsafe {
        let mut message = std::mem::zeroed::<MSG>();
        while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) != 0 {
            _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

/// Esegue un message loop guidato da `render`.
/// `render` viene chiamato quando non ci sono messaggi in coda;
/// ritorna `Ok(true)` per continuare a renderizzare immediatamente
/// (animazione continua) o `Ok(false)` per aspettare il prossimo messaggio
/// (event-driven). Ritorna quando la finestra viene chiusa o se `render`
/// ritorna un errore.
pub fn run_with<F>(mut render: F) -> Result<()>
where
    F: FnMut() -> std::result::Result<bool, Box<dyn std::error::Error>>,
{
    unsafe {
        let mut message = std::mem::zeroed::<MSG>();
        let mut animating = true;
        loop {
            if animating {
                while PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                    if message.message == WM_QUIT {
                        return Ok(());
                    }
                    _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            } else if GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) != 0 {
                if message.message == WM_QUIT {
                    return Ok(());
                }
                _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            } else {
                return Ok(());
            }
            animating = match render() {
                Ok(cont) => cont,
                Err(e) => {
                    eprintln!("Render error: {}", e);
                    false
                }
            };
        }
    }
}

/// Esegue un message loop bloccante con callback.
/// versione semplificata di `run_with` che non ritorna errori.
pub fn run_with_simple<F>(mut render: F)
where
    F: FnMut() -> bool,
{
    unsafe {
        let mut message = std::mem::zeroed::<MSG>();
        let mut animating = true;
        while animating {
            while PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                if message.message == WM_QUIT {
                    animating = false;
                    break;
                }
                _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            if animating && !render() {
                animating = false;
            }
        }
    }
}

/// Invia un messaggio di quit, causando l'uscita dal message loop.
pub fn quit() {
    unsafe {
        PostQuitMessage(0);
    }
}

/// Pompe tutti i messaggi pendenti senza bloccarsi.
/// Ritorna `false` se è stato ricevuto un WM_QUIT
/// (chiudere la pump), `true` altrimenti.
pub fn pump() -> bool {
    unsafe {
        let mut message = std::mem::zeroed::<MSG>();
        while PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
            if message.message == WM_QUIT {
                return false;
            }
            _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        true
    }
}

/// Pump con un timeout massimo in millisecondi.
/// Ritorna `true` se ci sono ancora messaggi da processare,
/// `false` se il timeout è scaduto o è arrivato WM_QUIT.
pub fn pump_with_timeout(timeout_ms: u32) -> bool {
    unsafe {
        let mut message = std::mem::zeroed::<MSG>();
        let result = MsgWaitForMultipleObjects(
            0,
            std::ptr::null(),
            0, // bWaitAll
            timeout_ms,
            0x0000_0004 | 0x0000_0100, // QS_ALLINPUT | QS_ALLPOSTMESSAGE
        );
        if result == 0xFFFFFFFF {
            return false; // errore
        }
        if result == 258 {
            // WAIT_TIMEOUT - nessun messaggio
            return true;
        }
        // Ci sono messaggi da processare
        while PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
            if message.message == WM_QUIT {
                return false;
            }
            _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        true
    }
}

// ============================================================================
// Internal Helpers
// ============================================================================

fn class_name() -> *const u16 {
    static NAME: OnceLock<Vec<u16>> = OnceLock::new();
    let name = NAME.get_or_init(|| "ru_wx.Window\0".encode_utf16().collect());
    name.as_ptr()
}

unsafe fn register_window_class() {
    static REGISTER: OnceLock<()> = OnceLock::new();
    REGISTER.get_or_init(|| unsafe {
        // Enable Per-Monitor V2 DPI awareness via SetProcessDpiAwarenessContext
        #[cfg(target_arch = "x86_64")]
        {
            use windows_sys::Win32::UI::HiDpi::{
                SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
            };
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }

        let class_name = class_name();
        let hinstance = GetModuleHandleW(std::ptr::null());
        let cursor = LoadCursorW(std::ptr::null_mut(), IDC_ARROW);
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: std::ptr::null_mut(),
            hCursor: cursor,
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name,
        };
        let _ = RegisterClassW(&wc);
    });
}

unsafe extern "system" fn wndproc(
    hwnd: HWND,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> LRESULT {
    let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State;
    let mut handled = None;

    if !state.is_null() {
        // Detach handlers before invoking (prevents reentrant aliasing)
        let mut message_handler = (*state).message.take();
        let mut resize_handler = (*state).resize.take();
        let mut paint_handler = (*state).paint.take();

        // Invoke message handler
        if let Some(handler) = message_handler.as_mut() {
            handled = handler(hwnd, message, wparam, lparam);
        }

        // Handle WM_SIZE
        if handled.is_none() && message == WM_SIZE {
            if let Some(handler) = resize_handler.as_mut() {
                let width = (lparam & 0xFFFF) as i32;
                let height = ((lparam >> 16) & 0xFFFF) as i32;
                handler(width, height);
                handled = Some(0);
            }
        }

        // Handle WM_PAINT
        if handled.is_none() && message == WM_PAINT {
            if let Some(handler) = paint_handler.as_mut() {
                let should_continue = handler();
                if !should_continue {
                    handled = Some(0);
                }
            }
        }

        // Check if window was destroyed during handler call (reentrant WM_NCDESTROY)
        let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State;
        if !state.is_null() {
            (*state).message = message_handler;
            (*state).resize = resize_handler;
            (*state).paint = paint_handler;
        }
    }

    // Clean up state on WM_NCDESTROY
    if message == WM_NCDESTROY {
        let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State;
        if !state.is_null() {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            drop(Box::from_raw(state));
        }
    }

    // Return handled result or default processing
    if let Some(result) = handled {
        return result;
    }

    match message {
        WM_DESTROY => {
            PostQuitMessage(0);
            0isize
        }
        WM_PAINT => {
            // Default paint: validate the update region
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            BeginPaint(hwnd, &mut ps);
            EndPaint(hwnd, &ps);
            0isize
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0isize
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

// ============================================================================
// Re-export commonly used constants and types
// ============================================================================

// Key WM message constants
pub const WM_CLOSE: u32 = windows_sys::Win32::UI::WindowsAndMessaging::WM_CLOSE;
pub const WM_DESTROY: u32 = windows_sys::Win32::UI::WindowsAndMessaging::WM_DESTROY;
pub const WM_PAINT: u32 = windows_sys::Win32::UI::WindowsAndMessaging::WM_PAINT;
pub const WM_SIZE: u32 = windows_sys::Win32::UI::WindowsAndMessaging::WM_SIZE;
pub const WM_QUIT: u32 = windows_sys::Win32::UI::WindowsAndMessaging::WM_QUIT;
pub const WM_NCDESTROY: u32 = windows_sys::Win32::UI::WindowsAndMessaging::WM_NCDESTROY;
pub const WM_KEYDOWN: u32 = windows_sys::Win32::UI::WindowsAndMessaging::WM_KEYDOWN;
pub const WM_KEYUP: u32 = windows_sys::Win32::UI::WindowsAndMessaging::WM_KEYUP;
pub const WM_MOUSEWHEEL: u32 = windows_sys::Win32::UI::WindowsAndMessaging::WM_MOUSEWHEEL;
pub const WM_LBUTTONDOWN: u32 = windows_sys::Win32::UI::WindowsAndMessaging::WM_LBUTTONDOWN;
pub const WM_RBUTTONDOWN: u32 = windows_sys::Win32::UI::WindowsAndMessaging::WM_RBUTTONDOWN;

// Window styles
pub const WS_OVERLAPPEDWINDOW: u32 =
    windows_sys::Win32::UI::WindowsAndMessaging::WS_OVERLAPPEDWINDOW;
pub const WS_CHILD: u32 = windows_sys::Win32::UI::WindowsAndMessaging::WS_CHILD;
pub const WS_VISIBLE: u32 = windows_sys::Win32::UI::WindowsAndMessaging::WS_VISIBLE;

// Common constants
pub const CW_USEDEFAULT: i32 = windows_sys::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT;
pub const SW_SHOW: i32 = windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOW;
pub const SW_HIDE: i32 = windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;
pub const PM_REMOVE: u32 = windows_sys::Win32::UI::WindowsAndMessaging::PM_REMOVE;
pub const SWP_NOZORDER: u32 = windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOZORDER;
pub const SWP_NOMOVE: u32 = windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOMOVE;
pub const SWP_NOSIZE: u32 = windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOSIZE;
pub const GWLP_USERDATA: i32 = windows_sys::Win32::UI::WindowsAndMessaging::GWLP_USERDATA;

// Class styles
pub const CS_HREDRAW: u32 = windows_sys::Win32::UI::WindowsAndMessaging::CS_HREDRAW;
pub const CS_VREDRAW: u32 = windows_sys::Win32::UI::WindowsAndMessaging::CS_VREDRAW;
