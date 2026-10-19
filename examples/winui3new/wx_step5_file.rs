//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Step 5 — File Dialog WinUI3/XAML
//!
//! Open/save con filtro estensioni (5.2.1–5.2.5). `XamlFileDialog` è oggi
//! uno stub (WinRT `FileOpenPicker` richiede bridging `IAsyncOperation`):
//! il demo mostra il flusso previsto e ne logga l'esito in console.
//!
//! Run with: cargo run --example wx_step5_file --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlButton, XamlFileDialog, XamlFrame, XamlTextBlock};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 5 — File Dialog")
        .with_size(600, 320)
        .with_padding(16.0)
        .with_spacing(12.0)
        .build();

    println!("=== Step 5: File Dialog ===");

    let _label = XamlTextBlock::new(&frame, "Percorso selezionato: (nessuno)");

    // Open con filtro estensioni (5.2.1 / 5.2.2)
    let btn_open = XamlButton::new(&frame, "Apri (*.txt;*.rs)");
    btn_open.set_width(220.0);
    btn_open.on_click(|| {
        let path = XamlFileDialog::open(&XamlFrame::builder().build(), "Apri file", "*.txt;*.rs");
        println!("[FILE] open → {:?}", path);
    });

    // Save con nome file (5.2.3)
    let btn_save = XamlButton::new(&frame, "Salva come…");
    btn_save.set_width(220.0);
    btn_save.on_click(|| {
        let path = XamlFileDialog::save(&XamlFrame::builder().build(), "Salva file", "*.txt");
        println!("[FILE] save → {:?}", path);
    });

    // 5.2.4 cartella iniziale + 5.2.5 multi-select: documentati, oggi stub
    let _info = XamlTextBlock::new(
        &frame,
        "Previsti: cartella iniziale suggerita e multi-select (oggi stub).",
    );

    println!("2 bottoni open/save. L'esito (o il log stub) compare in console.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
