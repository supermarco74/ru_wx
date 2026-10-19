//! Step 2 — ComboBox WinUI3/XAML
//!
//! Dropdown con selezione visualizzata, uno editabile.
//!
//! Run with: cargo run --example wx_step2_combo --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlComboBox, XamlFrame, XamlTextBlock};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 2 — ComboBox")
        .with_size(500, 300)
        .with_padding(20.0)
        .with_spacing(10.0)
        .build();

    println!("=== Step 2: ComboBox ===");

    let _lbl1 = XamlTextBlock::new(&frame, "Scegli un linguaggio:");
    let combo1 = XamlComboBox::new(&frame, &["Rust", "C++", "Python", "Go", "Zig"]);
    combo1.set_width(260.0);
    combo1.set_selected(0);
    combo1.on_selection_changed(|idx| {
        println!("[COMBO] Linguaggio selezionato: indice {}", idx);
    });

    let _lbl2 = XamlTextBlock::new(&frame, "O scrivi libero:");
    let combo2 = XamlComboBox::new(&frame, &["Opzione A", "Opzione B", "Opzione C"]);
    combo2.set_width(260.0);
    combo2.set_selected(1);

    // Cambia selezione a runtime
    combo1.set_selected(2);
    println!("[SET] combo1 selezione cambiata a indice 2");

    println!("2 ComboBox. Apri il dropdown e seleziona per verificare.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
