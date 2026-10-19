//! Step 3 — Splitter WinUI3/XAML
//!
//! Pannello sinistra (tree simulato), contenuto destra, drag per ridimensionare.
//!
//! Run with: cargo run --example wx_step3_splitter --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlFrame, XamlSplitter, XamlTextBlock};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 3 — Splitter")
        .with_size(700, 400)
        .build();

    println!("=== Step 3: Splitter ===");

    // Pane sinistro (tree simulato)
    let tree = XamlTextBlock::new(
        &frame,
        "Pannello sinistro\n\n- Item 1\n- Item 2\n  - Sub 2a\n  - Sub 2b\n- Item 3",
    );

    // Contenuto destro
    let content = XamlTextBlock::new(
        &frame,
        "Contenuto principale\n\nTrascina il bordo per ridimensionare.",
    );

    let splitter = XamlSplitter::new(
        &frame,
        ru_wx::platform::winui3_xaml::element_bridge::WidgetDesc::TextBlock {
            id: tree.id(),
            text: "Pannello sinistro\n\n- Item 1\n- Item 2\n  - Sub 2a\n  - Sub 2b\n- Item 3"
                .into(),
            font_size: 0.0,
            bold: false,
        },
        ru_wx::platform::winui3_xaml::element_bridge::WidgetDesc::TextBlock {
            id: content.id(),
            text: "Contenuto principale\n\nTrascina il bordo per ridimensionare.".into(),
            font_size: 0.0,
            bold: false,
        },
    );
    splitter.set_pane_length(200.0);

    println!("Splitter con pane 200px. Trascina per ridimensionare.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
