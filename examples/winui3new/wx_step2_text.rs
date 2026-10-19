//! Step 2 — TextBlock WinUI3/XAML
//!
//! 5 TextBlock con stili diversi, uno aggiornato a runtime.
//!
//! Run with: cargo run --example wx_step2_text --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlFrame, XamlTextBlock};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 2 — TextBlock")
        .with_size(500, 350)
        .with_padding(20.0)
        .with_spacing(8.0)
        .build();

    println!("=== Step 2: TextBlock ===");

    let _t1 = XamlTextBlock::new(&frame, "Testo normale (default)");
    let t2 = XamlTextBlock::new(&frame, "Testo grande");
    t2.set_font_size(28.0);
    let t3 = XamlTextBlock::new(&frame, "Testo in grassetto");
    t3.set_bold(true);
    let t4 = XamlTextBlock::new(&frame, "Grande e grassetto");
    t4.set_font_size(22.0);
    t4.set_bold(true);
    let t5 = XamlTextBlock::new(&frame, "Questo testo cambia!");
    t5.set_font_size(16.0);

    // Cambia il testo a runtime
    t5.set_label("Testo aggiornato a runtime!");
    println!("[SET] t5.set_label() — testo cambiato");

    println!("5 TextBlock con stili diversi. Verifica visiva.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
