//! Step 3 — Tab WinUI3/XAML
//!
//! 3 tab con contenuto diverso, log del cambio tab.
//!
//! Run with: cargo run --example wx_step3_tab --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlButton, XamlFrame, XamlTab, XamlTextBlock};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 3 — Tab")
        .with_size(600, 400)
        .with_padding(16.0)
        .build();

    println!("=== Step 3: Tab ===");

    let tab = XamlTab::new(&frame);

    // Tab 1: Testo
    let t1 = XamlTextBlock::new(
        &frame,
        "Contenuto del Tab 1\n\nQuesto e' un semplice testo.",
    );
    tab.add_page(
        "Tab 1",
        ru_wx::platform::winui3_xaml::element_bridge::WidgetDesc::TextBlock {
            id: t1.id(),
            text: "Contenuto del Tab 1\n\nQuesto e' un semplice testo.".into(),
            font_size: 0.0,
            bold: false,
        },
    );

    // Tab 2: Bottone
    let btn = XamlButton::new(&frame, "Bottone nel Tab 2");
    btn.on_click(|| println!("[CLICK] Bottone Tab 2"));
    tab.add_page(
        "Tab 2",
        ru_wx::platform::winui3_xaml::element_bridge::WidgetDesc::Button {
            id: btn.id(),
            label: "Bottone nel Tab 2".into(),
            enabled: true,
            width: 0.0,
        },
    );

    // Tab 3: Closable
    let t3 = XamlTextBlock::new(&frame, "Questo tab si puo' chiudere!");
    tab.add_page_closable(
        "Tab 3",
        ru_wx::platform::winui3_xaml::element_bridge::WidgetDesc::TextBlock {
            id: t3.id(),
            text: "Questo tab si puo' chiudere!".into(),
            font_size: 0.0,
            bold: false,
        },
    );

    tab.set_selected(0);
    println!("3 tab creati. Cambia tab per verificare.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
