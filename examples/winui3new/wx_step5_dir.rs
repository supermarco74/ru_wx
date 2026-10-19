//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Step 5 — Dir Dialog WinUI3/XAML
//!
//! Directory picker (5.5.1). `XamlDirDialog` è oggi uno stub (WinRT
//! `FolderPicker` richiede bridging `IAsyncOperation`): il demo mostra il
//! flusso previsto e ne logga l'esito in console.
//!
//! Run with: cargo run --example wx_step5_dir --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlButton, XamlDirDialog, XamlFrame, XamlTextBlock};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 5 — Dir Dialog")
        .with_size(550, 280)
        .with_padding(16.0)
        .with_spacing(12.0)
        .build();

    println!("=== Step 5: Dir Dialog ===");

    let _label = XamlTextBlock::new(&frame, "Cartella selezionata: (nessuna)");

    let btn_dir = XamlButton::new(&frame, "Scegli cartella…");
    btn_dir.set_width(220.0);
    btn_dir.on_click(|| {
        let path = XamlDirDialog::show(&XamlFrame::builder().build());
        println!("[DIR] show → {:?}", path);
    });

    let _info = XamlTextBlock::new(
        &frame,
        "Previsto: FolderPicker con cartella iniziale (oggi stub).",
    );

    println!("1 bottone folder picker. L'esito (o il log stub) compare in console.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
