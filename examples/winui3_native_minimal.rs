//! Esempio WinUI3 nativo - Nessun controllo, X bloccata
//!
//! Questo esempio dimostra:
//! - Finestra WinUI3 creata con windows_reactor (WinAppSDK + WinRT)
//! - La X NON chiude la finestra ( WM_CLOSE ignorato )
//! - Zero controlli interattivi (nessun button, input, slider)
//! - Solo contenuto statico: TextBlock

#![cfg(target_os = "windows")]

use windows_reactor::*;

/// Contenuto della finestra - solo testo statico
fn content(_cx: &mut RenderCx) -> Element {
    vstack((
        text_block("═══════════════════════════════════════════"),
        text_block("  WINUI3 NATIVE - ZERO CONTROLLI            "),
        text_block("═══════════════════════════════════════════"),
        text_block(""),
        text_block("La X in alto a destra e' DISATTIVATA"),
        text_block("Questa finestra NON si chiude cliccando X"),
        text_block(""),
        text_block("Backend: windows_reactor (WinAppSDK/WinRT)"),
        text_block("Nessun widget interattivo presente"),
        text_block(""),
        text_block("Per chiudere: Task Manager -> Termina"),
    ))
    .spacing(6.0)
    .into()
}

fn main() -> Result<()> {
    println!("[winui3_native_minimal] Avvio finestra WinUI3...");

    // Inizializza WinAppSDK runtime
    bootstrap()?;

    // Crea app con contenuto
    App::new()
        .title("WinUI3 Native - No Close, Zero Controls")
        .inner_size(500.0, 380.0)
        .backdrop(Backdrop::Mica)
        .render(content)?;

    Ok(())
}
