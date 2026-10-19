//! Step 2 — Slider e ProgressBar WinUI3/XAML
//!
//! Slider che pilota una progress bar, con valore numerico su stdout.
//!
//! Run with: cargo run --example wx_step2_slider --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{
    XamlApp, XamlFrame, XamlProgressBar, XamlSlider, XamlTextBlock,
};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 2 — Slider e ProgressBar")
        .with_size(500, 300)
        .with_padding(20.0)
        .with_spacing(14.0)
        .build();

    println!("=== Step 2: Slider e ProgressBar ===");

    let lbl = XamlTextBlock::new(&frame, "Valore: 50");
    lbl.set_font_size(18.0);

    let slider = XamlSlider::new(&frame, 0.0, 100.0, 50.0);
    let progress = XamlProgressBar::new(&frame, 100.0);
    progress.set_value(50.0);

    let _progress_ind = XamlProgressBar::indeterminate(&frame);

    // Cambia valore a runtime
    slider.set_value(75.0);
    progress.set_value(75.0);
    lbl.set_label("Valore: 75");
    println!("[SET] slider=75, progress=75");

    println!("Slider + ProgressBar determinata + indeterminata. Trascina per verificare.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
