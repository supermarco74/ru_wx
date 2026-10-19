//! Step 6 — Icone e immagini WinUI3/XAML
//!
//! Immagine da file, icona applicazione, e bitmap button.
//!
//! Run with: cargo run --example wx_step6_icon --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlFrame, XamlImage, XamlTextBlock};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 6 — Icone e immagini")
        .with_size(500, 400)
        .with_padding(16.0)
        .with_spacing(12.0)
        .build();

    println!("=== Step 6: Icone e immagini ===");

    let _label = XamlTextBlock::new(&frame, "Immagine da file (se esiste):");

    // Path assoluto: l'exe viene lanciata da altre cwd, il path relativo
    // "assets/..." non risolverebbe mai.
    // PNG e non SVG: WinUI3 (`BitmapImage`) non decodifica SVG.
    let img_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/ribbon_icons/save.png"
    );
    if std::path::Path::new(img_path).exists() {
        let img = XamlImage::new(&frame, img_path);
        // Size esplicita: senza, l'Image occupa tutta la larghezza del
        // vstack e schiaccia la label successiva fuori dalla finestra.
        img.set_size(96.0, 96.0);
        println!("[IMAGE] Caricata: {}", img_path);
    } else {
        println!("[IMAGE] File non trovato: {} (uso placeholder)", img_path);
        let _placeholder = XamlTextBlock::new(&frame, "[Immagine non disponibile]");
    }

    // Seconda immagine (placeholder)
    let _label2 = XamlTextBlock::new(&frame, "Icona applicazione: gestita da reactor App");

    println!("Immagine caricata. Verifica visiva.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
