//! Step 1 — Finestra base WinUI3/XAML
//!
//! Dimostra la creazione di una finestra XAML nativa con titolo e dimensione.
//! Verifica: la finestra si apre, mostra il titolo corretto, si chiude con X.
//!
//! Run with: cargo run --example wx_step1_window --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlFrame};

fn main() {
    let app = XamlApp::new();

    let frame = XamlFrame::builder()
        .with_title("Step 1 — Finestra base")
        .with_size(640, 480)
        .build();

    // Log per verifica
    println!("=== Step 1: Finestra base ===");
    println!("Titolo: {}", frame.title());
    let (w, h) = frame.size();
    println!("Dimensione: {}x{}", w, h);
    println!("ID frame: {}", frame.id());
    println!("Chiudi la finestra per terminare.");

    match app.run(frame) {
        Ok(_) => println!("Finestra chiusa correttamente."),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
