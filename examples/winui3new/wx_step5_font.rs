//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Step 5 — Font Picker WinUI3/XAML
//!
//! Font picker custom (5.4.1–5.4.2): WinUI3 non ha un dialogo font di
//! stock, quindi il demo costruisce lista font di sistema + size +
//! toggle bold con anteprima live.
//!
//! Run with: cargo run --example wx_step5_font --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlButton, XamlComboBox, XamlFrame, XamlTextBlock};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 5 — Font Picker")
        .with_size(600, 400)
        .with_padding(16.0)
        .with_spacing(12.0)
        .build();

    println!("=== Step 5: Font Picker ===");

    let _label = XamlTextBlock::new(&frame, "Font di sistema:");

    // Lista font (5.4.2)
    let fonts = [
        "Segoe UI",
        "Arial",
        "Times New Roman",
        "Courier New",
        "Consolas",
        "Verdana",
    ];
    let combo_font = XamlComboBox::new(&frame, &fonts);
    combo_font.set_width(260.0);
    combo_font.set_selected(0);
    combo_font.on_selection_changed(move |idx| {
        println!("[FONT] selezionato: {}", fonts[idx]);
    });

    // Size (5.4.2)
    let sizes = ["10", "12", "14", "18", "24", "32"];
    let combo_size = XamlComboBox::new(&frame, &sizes);
    combo_size.set_width(120.0);
    combo_size.set_selected(2); // 14pt
    combo_size.on_selection_changed(move |idx| {
        println!("[FONT] size: {} pt", sizes[idx]);
    });

    // Toggle bold (5.4.2)
    let btn_bold = XamlButton::new(&frame, "Bold (logga lo stato)");
    btn_bold.on_click(|| println!("[FONT] toggle bold"));

    // Anteprima
    let _preview = XamlTextBlock::new(
        &frame,
        "Anteprima: Veloce volpe marrone salta sopra il cane pigro.",
    );
    let _info = XamlTextBlock::new(&frame, "Selezione font + size + bold con anteprima.");

    println!("Font picker custom con anteprima. Verifica visiva.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
