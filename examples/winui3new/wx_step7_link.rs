//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Step 7 — Hyperlink WinUI3/XAML
//!
//! `XamlHyperlink` → `HyperlinkButton` (7.4.1–7.4.2): link che aprono
//! l'URL nel browser, più aggiornamento dinamico con `update()`.
//!
//! Run with: cargo run --example wx_step7_link --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlFrame, XamlHyperlink, XamlTextBlock};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 7 — Hyperlink")
        .with_size(500, 300)
        .with_padding(16.0)
        .with_spacing(12.0)
        .build();

    println!("=== Step 7: Hyperlink ===");

    let _label = XamlTextBlock::new(&frame, "Link (cliccanti):");

    // Link statici (7.4.1)
    let _link_docs = XamlHyperlink::new(&frame, "Documentazione ru_wx", "https://docs.rs/ru_wx");
    let _link_site = XamlHyperlink::new(&frame, "EasyTaskFlow", "https://www.easytaskflow.app");

    // Aggiornamento dinamico prima del render (omologo hyperlink_update)
    let link_dyn = XamlHyperlink::new(&frame, "Link temporaneo", "https://example.com");
    link_dyn.update("Sito aggiornato", "https://www.rust-lang.org");

    let _info = XamlTextBlock::new(
        &frame,
        "3 hyperlink: il terzo è stato aggiornato con update() prima del render.",
    );

    println!("3 hyperlink. Clicca per aprire l'URL nel browser default.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
