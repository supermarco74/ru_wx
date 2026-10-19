//! Step 3 — Panel WinUI3/XAML
//!
//! Panel annidati con contenuto diverso.
//!
//! Run with: cargo run --example wx_step3_panel --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::element_bridge::WidgetDesc;
use ru_wx::platform::winui3_xaml::{XamlApp, XamlButton, XamlFrame, XamlPanel, XamlTextBlock};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 3 — Panel")
        .with_size(500, 400)
        .with_padding(16.0)
        .with_spacing(12.0)
        .build();

    println!("=== Step 3: Panel ===");

    // Panel esterno con titolo
    let outer = XamlPanel::new(&frame);
    let title = XamlTextBlock::new(&frame, "Panel esterno");
    title.set_font_size(20.0);
    title.set_bold(true);
    outer.add_child(WidgetDesc::TextBlock {
        id: title.id(),
        text: "Panel esterno".into(),
        font_size: 20.0,
        bold: true,
    });

    // Panel interno con bottoni, annidato dentro il panel esterno
    let inner = XamlPanel::new(&frame);
    let btn1 = XamlButton::new(&frame, "Bottone 1");
    let btn2 = XamlButton::new(&frame, "Bottone 2");
    btn1.on_click(|| println!("[CLICK] Bottone 1"));
    btn2.on_click(|| println!("[CLICK] Bottone 2"));
    inner.add_child(WidgetDesc::Button {
        id: btn1.id(),
        label: "Bottone 1".into(),
        enabled: true,
        width: 0.0,
    });
    inner.add_child(WidgetDesc::Button {
        id: btn2.id(),
        label: "Bottone 2".into(),
        enabled: true,
        width: 0.0,
    });
    outer.add_child(
        inner
            .take_desc()
            .expect("descrittore del panel interno nel frame"),
    );

    println!("2 Panel annidati con bottoni. Verifica visiva e click.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
