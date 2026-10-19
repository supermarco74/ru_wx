//! Step 3 — BoxSizer WinUI3/XAML
//!
//! Layout verticale con header, contenuto stretch, footer.
//!
//! Run with: cargo run --example wx_step3_sizer --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::element_bridge::WidgetDesc;
use ru_wx::platform::winui3_xaml::{XamlApp, XamlBoxSizer, XamlButton, XamlFrame, XamlTextBlock};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 3 — BoxSizer")
        .with_size(500, 400)
        .with_padding(16.0)
        .build();

    println!("=== Step 3: BoxSizer ===");

    // Sizer verticale principale
    let mut main_sizer = XamlBoxSizer::vertical(&frame);
    main_sizer.set_spacing(8.0);

    // Header
    let header = XamlTextBlock::new(&frame, "Header (in alto)");
    header.set_font_size(18.0);
    header.set_bold(true);
    main_sizer.add(WidgetDesc::TextBlock {
        id: header.id(),
        text: "Header (in alto)".into(),
        font_size: 18.0,
        bold: true,
    });

    // Contenuto centrale (stretch)
    let content = XamlTextBlock::new(&frame, "Contenuto centrale (stretch)");
    main_sizer.add(WidgetDesc::TextBlock {
        id: content.id(),
        text: "Contenuto centrale (stretch)".into(),
        font_size: 0.0,
        bold: false,
    });

    // Footer con bottoni orizzontali
    let mut footer_sizer = XamlBoxSizer::horizontal(&frame);
    footer_sizer.set_spacing(8.0);

    let btn_ok = XamlButton::new(&frame, "OK");
    let btn_cancel = XamlButton::new(&frame, "Annulla");
    btn_ok.on_click(|| println!("[CLICK] OK"));
    btn_cancel.on_click(|| println!("[CLICK] Annulla"));
    footer_sizer.add(WidgetDesc::Button {
        id: btn_ok.id(),
        label: "OK".into(),
        enabled: true,
        width: 0.0,
    });
    footer_sizer.add(WidgetDesc::Button {
        id: btn_cancel.id(),
        label: "Annulla".into(),
        enabled: true,
        width: 0.0,
    });

    // Annida il footer (orizzontale) nel sizer principale
    main_sizer.add(
        footer_sizer
            .take_desc()
            .expect("descrittore del footer nel frame"),
    );

    println!("Layout: header, contenuto stretch, footer con bottoni. Verifica visiva.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
