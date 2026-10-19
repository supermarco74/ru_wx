//! Step 4 — Context Menu WinUI3/XAML
//!
//! Right-click su un elemento mostra un menu contestuale.
//! Nota: il context menu in WinUI3 usa MenuFlyout ancorato all'elemento.
//!
//! Run with: cargo run --example wx_step4_context --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlButton, XamlFrame, XamlTextBlock};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 4 — Context Menu")
        .with_size(500, 300)
        .with_padding(16.0)
        .with_spacing(12.0)
        .build();

    println!("=== Step 4: Context Menu ===");

    let _label = XamlTextBlock::new(
        &frame,
        "Fai right-click sul bottone per il menu contestuale.",
    );

    // In WinUI3 il context menu e' un MenuFlyout attaccato al widget.
    // Per ora dimostriamo il concetto con un bottone che ha un flyout.
    let btn = XamlButton::new(&frame, "Right-click me (o click)");
    btn.on_click(|| {
        println!("[CONTEXT] Menu contestuale aperto (simulato)");
        println!("  - Opzione 1");
        println!("  - Opzione 2");
        println!("  - Opzione 3");
    });

    println!("Bottone con menu contestuale simulato. Clicca per verificare.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
