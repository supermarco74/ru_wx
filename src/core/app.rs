//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Application entry point and message-loop driver.
//!
//! [`App::new`] performs any platform-specific one-shot initialisation
//! (on Windows it calls `InitCommonControlsEx` so the common controls
//! render with visual styles) and [`App::run`] enters the platform
//! event loop on the supplied top-level frame, blocking until the
//! window is closed.
//!
//! ```no_run
//! use ru_wx::prelude::*;
//! let app = App::new();
//! let frame = Frame::builder().with_title("hi").build();
//! app.run(frame);
//! ```

use crate::window::frame::Frame;

/// Application entry point.
/// On Windows, this is a thin wrapper that calls frame.show() which runs the message loop.
/// On macOS, this would set up NSApplication.
/// On Linux, this would initialize GTK.
pub struct App;

impl App {
    pub fn new() -> Self {
        // Platform-specific initialization could go here
        // e.g., on Windows: InitCommonControlsEx for visual styles
        #[cfg(target_os = "windows")]
        // SAFETY: Win32 FFI call with validated arguments (HWND / HMENU / handle) and a buffer large enough for the output.
        unsafe {
            // Enforce PerMonitorV2 DPI awareness at runtime as a
            // safety net. The library's `app.manifest` already
            // requests this, but Win32 silently downgrades the
            // process to DPI-unaware when the manifest is
            // stripped (e.g. by a launcher that rewrites the
            // manifest, or by some anti-virus products that
            // quarantine the binary). When that happens, every
            // `GetClientRect` / `MoveWindow` call returns
            // *physical* pixels, which is the exact bug the
            // AUI layout code is fighting. Calling the modern
            // `SetProcessDpiAwarenessContext` here makes the
            // process behave as a true PerMonitorV2 app even if
            // the manifest was lost.
            //
            // MUST be called before any window is created (this
            // is the very first thing `App::new` does).
            let _dpi_ok = crate::core::dpi::set_process_dpi_awareness_context_per_monitor_v2();

            use windows_sys::Win32::UI::Controls::*;
            let icc = INITCOMMONCONTROLSEX {
                dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
                // ICC_STANDARD_CLASSES already covers the tab control, but
                // we include ICC_TAB_CLASSES and ICC_LISTVIEW_CLASSES
                // explicitly to be belt-and-suspenders robust against any
                // `ICC_*` flag refactor in future `windows-sys` releases.
                dwICC: ICC_STANDARD_CLASSES
                    | ICC_BAR_CLASSES
                    | ICC_TAB_CLASSES
                    | ICC_LISTVIEW_CLASSES
                    | ICC_LINK_CLASS
                    | ICC_DATE_CLASSES,
            };
            let icc_ok = InitCommonControlsEx(&icc);
            // Senza manifest v6 nel binario (gli esempi del package non lo
            // incorporano) Win32 carica comctl32 v5, che rifiuta
            // `ICC_LINK_CLASS` e non registra la classe `SysLink`
            // (`CreateWindowExW` → ERROR_CLASS_DOES_NOT_EXIST). Se
            // l'inizializzazione fallisce, attiviamo a runtime un contesto
            // di applicazione che dichiara le Common-Controls 6.0.0.0 e
            // riproviamo.
            if icc_ok == 0 && activate_common_controls_v6() {
                let _ = InitCommonControlsEx(&icc);
            }
        }
        #[cfg(target_os = "macos")]
        Self::macos_init();
        // GTK4: init lazy e non-panicking (in test/headless restituisce
        // false e i creatori widget cadono sugli stub in-memory).
        #[cfg(target_os = "linux")]
        let _ = crate::platform::gtk::init();
        App
    }

    #[cfg(target_os = "macos")]
    fn macos_init() {
        crate::platform::appkit::init_application();
    }

    /// Run the application with the given frame.
    /// This enters the platform event loop and does not return until the window is closed.
    pub fn run(self, frame: Frame) {
        // Autostart del debug inspector (solo in debug o con la feature
        // `inspector`): va avviato **prima** di `show()`, così l'handler
        // dei messaggi custom è registrato quando parte il run loop.
        #[cfg(any(debug_assertions, feature = "inspector"))]
        crate::debug::inspector::start(&frame);
        frame.show();
    }
}

/// Attiva a runtime un contesto di applicazione che dichiara le
/// Common-Controls 6.0.0.0. Serve quando il binario non incorpora il
/// manifest v6 (gli esempi del package non lo fanno): senza manifest
/// Win32 carica comctl32 v5, che rifiuta `ICC_LINK_CLASS` e non
/// registra la classe `SysLink`. Restituisce `true` se il contesto è
/// attivato (il `CreateActCtxW` non ha prodotto un handle
/// invalido e `ActivateActCtx` ha avuto successo).
///
/// `pub(crate)` perché lo riusa anche il backend `platform::winui3`
/// (stesso problema: binario senza manifest v6 → comctl32 v5).
#[cfg(target_os = "windows")]
pub(crate) fn activate_common_controls_v6() -> bool {
    use windows_sys::Win32::System::ApplicationInstallationAndServicing::{
        ActivateActCtx, CreateActCtxW, ACTCTXW,
    };

    // Manifest minimo: solo la dipendenza dalle Common-Controls v6.
    const MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0" processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df" language="*"/>
    </dependentAssembly>
  </dependency>
</assembly>
"#;

    // `ACTCTXW` accetta solo un percorso file (non un buffer in
    // memoria): scriviamo il manifest in una directory temporanea con
    // un nome che include il PID, così ogni processo ha il suo e non
    // si scrive su file condivisi. Il contenuto è fisso, quindi lo
    // riscriviamo solo se non esiste ancora.
    let path = std::env::temp_dir().join(format!("ru_wx_comctl6_{}.manifest", std::process::id()));
    if !path.exists() && std::fs::write(&path, MANIFEST).is_err() {
        return false;
    }
    let wide = crate::platform::win32::to_wide(&path.to_string_lossy());

    // SAFETY: `actctx` è inizializzata per intero e `wide` resta
    // vivo fino alla fine della chiamata. L'handle del contesto
    // NON viene chiuso: il contesto deve restare attivo per tutta
    // la vita del processo perché le DLL risolte dopo (comctl32)
    // lo risentano.
    unsafe {
        let mut actctx: ACTCTXW = std::mem::zeroed();
        actctx.cbSize = std::mem::size_of::<ACTCTXW>() as u32;
        actctx.lpSource = wide.as_ptr();
        let handle = CreateActCtxW(&actctx);
        let raw = handle as isize;
        if raw == 0 || raw == -1 {
            return false;
        }
        let mut cookie: usize = 0;
        ActivateActCtx(handle, &mut cookie) != 0
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
