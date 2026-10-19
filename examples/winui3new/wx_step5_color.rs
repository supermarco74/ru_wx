//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Step 5 — Color Picker WinUI3/XAML
//!
//! `XamlColorDialog` inline nel layout (5.3.1–5.3.3): colore iniziale,
//! `set_color` a runtime, show/close per liberare lo spazio.
//!
//! Run with: cargo run --example wx_step5_color --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlColorDialog, XamlFrame, XamlTextBlock};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 5 — Color Picker")
        .with_size(600, 420)
        .with_padding(16.0)
        .with_spacing(12.0)
        .build();

    println!("=== Step 5: Color Picker ===");

    // Colore iniziale = accent (5.3.2)
    let _label = XamlTextBlock::new(&frame, "Colore iniziale: RGB(79, 70, 229)");

    let dlg = XamlColorDialog::new(&frame, 79, 70, 229);

    // Cambio colore programmatico prima del render (5.3.3 → RGB restituito)
    dlg.set_color(200, 40, 40);
    let _label2 = XamlTextBlock::new(&frame, "Dopo set_color(200, 40, 40): rosso");

    // Il picker è un widget inline: show() lo espande, lo() lo comprime.
    dlg.show();

    let _info = XamlTextBlock::new(
        &frame,
        "Picker inline: show()/close() gestiscono la visibilità, set_color() il valore.",
    );

    println!("Picker RGB visibile nel layout. Verifica visiva.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
