//! Step 7 — ListCtrl WinUI3/XAML
//!
//! Tabella 4 colonne con dati e selezione.
//!
//! Run with: cargo run --example wx_step7_listctrl --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlFrame, XamlListCtrl, XamlTextBlock};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 7 — ListCtrl")
        .with_size(600, 400)
        .with_padding(16.0)
        .with_spacing(12.0)
        .build();

    println!("=== Step 7: ListCtrl ===");

    let _label = XamlTextBlock::new(&frame, "Tabella prodotti:");

    let list = XamlListCtrl::new(&frame);
    list.add_column("ID");
    list.add_column("Nome");
    list.add_column("Prezzo");
    list.add_column("Stock");

    list.add_row(&["1", "ru_wx crate", "€0.00", "999"]);
    list.add_row(&["2", "windows-sys", "€0.00", "999"]);
    list.add_row(&["3", "resvg", "€0.00", "999"]);
    list.add_row(&["4", "image", "€0.00", "999"]);
    list.add_row(&["5", "serde", "€0.00", "999"]);

    println!("ListCtrl con 4 colonne e 5 righe. Verifica visiva.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
