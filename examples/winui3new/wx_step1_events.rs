//! Step 1 — Eventi finestra WinUI3/XAML
//!
//! Dimostra la registrazione e il dispatch degli eventi finestra:
//! - `on_close`: chiamato quando la finestra si chiude
//! - `on_resize`: chiamato quando la finestra viene ridimensionata
//! - `set_title`: cambio titolo a runtime
//!
//! Verifica: apri la finestra, ridimensiona, chiudi. Gli eventi loggano su stdout.
//!
//! Run with: cargo run --example wx_step1_events --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlFrame};

fn main() {
    let app = XamlApp::new();

    let frame = XamlFrame::builder()
        .with_title("Step 1 — Eventi finestra")
        .with_size(640, 480)
        .build();

    println!("=== Step 1: Eventi finestra ===");
    println!("Titolo iniziale: {}", frame.title());

    // Cambia titolo prima di avviare (dimostra set_title)
    frame.set_title("Step 1 — Titolo aggiornato!");
    println!("[EVENT] set_title — titolo cambiato a: {}", frame.title());

    // Registra handler close
    frame.on_close(|| {
        println!("[EVENT] on_close — finestra chiusa");
    });

    // Registra handler resize
    frame.on_resize(|w, h| {
        println!("[EVENT] on_resize — {}x{}", w, h);
    });

    let (w, h) = frame.size();
    println!("Dimensione: {}x{}", w, h);
    println!("Apri la finestra, ridimensiona per vedere on_resize, poi chiudi.");

    match app.run(frame) {
        Ok(_) => println!("Applicazione terminata correttamente."),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
