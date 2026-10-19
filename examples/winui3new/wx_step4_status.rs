//! Step 4 — StatusBar WinUI3/XAML
//!
//! Status bar con 3 campi, aggiornamento a runtime.
//!
//! Run with: cargo run --example wx_step4_status --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlFrame, XamlStatusBar, XamlTextBlock};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 4 — StatusBar")
        .with_size(600, 300)
        .with_padding(16.0)
        .with_spacing(12.0)
        .build();

    println!("=== Step 4: StatusBar ===");

    let _content = XamlTextBlock::new(&frame, "Contenuto principale della finestra.");

    let statusbar = XamlStatusBar::new(&frame, 3);
    statusbar.set_status_text("Ready", 0);
    statusbar.set_status_text("Row: 1, Col: 1", 1);
    statusbar.set_status_text("UTF-8", 2);

    // Aggiorna a runtime
    statusbar.set_status_text("Modified", 0);
    println!("[SET] statusbar campo 0 aggiornato a 'Modified'");

    println!("StatusBar con 3 campi. Verifica visiva in fondo alla finestra.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
