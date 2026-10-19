//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Step 6 — BitmapButton WinUI3/XAML
//!
//! Bottoni con icona (6.3.1–6.3.3): ogni riga è un `HStack` con `Image`
//! (PNG da `examples/ribbon_icons/`) + `Button`, più uno stato disabled.
//!
//! Run with: cargo run --example wx_step6_bitmap --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::element_bridge::{ImageStretch, WidgetDesc};
use ru_wx::platform::winui3_xaml::{
    XamlApp, XamlBoxSizer, XamlButton, XamlFrame, XamlImage, XamlTextBlock,
};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 6 — BitmapButton")
        .with_size(480, 400)
        .with_padding(16.0)
        .with_spacing(10.0)
        .build();

    println!("=== Step 6: BitmapButton ===");

    let _label = XamlTextBlock::new(&frame, "Bottoni con icona (PNG 24×24):");

    // 4 bottoni con icone diverse (6.3.1)
    let rows: [(&str, &str); 4] = [
        ("new", "Nuovo"),
        ("open", "Apri"),
        ("save", "Salva"),
        ("undo", "Annulla"),
    ];
    for (icon, btn_label) in rows {
        let uri = format!(
            "{}/examples/ribbon_icons/{}.png",
            env!("CARGO_MANIFEST_DIR"),
            icon
        );
        if !std::path::Path::new(&uri).exists() {
            println!("[BITMAP] Icona mancante: {uri}");
            continue;
        }

        let mut row = XamlBoxSizer::horizontal(&frame);
        row.set_spacing(8.0);

        let img = XamlImage::new(&frame, &uri);
        img.set_size(24.0, 24.0);
        row.add(WidgetDesc::Image {
            id: img.id(),
            uri,
            stretch: ImageStretch::Uniform,
            width: Some(24.0),
            height: Some(24.0),
        });

        let btn = XamlButton::new(&frame, btn_label);
        btn.set_width(160.0);
        btn.on_click(move || println!("[BITMAP BTN] {btn_label}"));
        row.add(WidgetDesc::Button {
            id: btn.id(),
            label: btn_label.into(),
            enabled: true,
            width: 160.0,
        });
    }

    // Stato disabled (6.3.3)
    let btn_disabled = XamlButton::new(&frame, "Disabilitato (hover/pressed off)");
    btn_disabled.set_width(240.0);
    btn_disabled.set_enabled(false);

    println!("4 bottoni con icona + 1 disabled. Verifica visiva.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
