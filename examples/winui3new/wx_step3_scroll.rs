//! Step 3 — ScrollViewer WinUI3/XAML
//!
//! 50 righe di testo in un'area scrollabile.
//!
//! Run with: cargo run --example wx_step3_scroll --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlFrame, XamlScrolledWindow, XamlTextBlock};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 3 — ScrollViewer")
        .with_size(500, 300)
        .with_padding(16.0)
        .build();

    println!("=== Step 3: ScrollViewer ===");

    // Genera 50 righe di testo
    let mut text = String::from("Contenuto scrollabile (50 righe):\n\n");
    for i in 1..=50 {
        text.push_str(&format!("Riga {} — Lorem ipsum dolor sit amet\n", i));
    }

    let content = XamlTextBlock::new(&frame, &text);
    let scroll = XamlScrolledWindow::new(
        &frame,
        ru_wx::platform::winui3_xaml::element_bridge::WidgetDesc::TextBlock {
            id: content.id(),
            text,
            font_size: 0.0,
            bold: false,
        },
    );
    scroll.set_vertical_scroll(true);

    println!("50 righe in ScrollViewer. Scorri per verificare.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
