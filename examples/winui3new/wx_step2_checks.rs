//! Step 2 — CheckBox e RadioButton WinUI3/XAML
//!
//! 3 checkbox indipendenti + 3 radio in gruppo, stato visualizzato su stdout.
//!
//! Run with: cargo run --example wx_step2_checks --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{
    XamlApp, XamlCheckBox, XamlFrame, XamlRadioButton, XamlTextBlock,
};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 2 — CheckBox e RadioButton")
        .with_size(500, 400)
        .with_padding(20.0)
        .with_spacing(8.0)
        .build();

    println!("=== Step 2: CheckBox e RadioButton ===");

    // Checkbox
    let lbl1 = XamlTextBlock::new(&frame, "Opzioni:");
    lbl1.set_font_size(16.0);
    lbl1.set_bold(true);

    let cb1 = XamlCheckBox::new(&frame, "Abilita notifiche", true);
    cb1.on_toggled(|checked| println!("[CHECK] Notifiche: {}", checked));

    let cb2 = XamlCheckBox::new(&frame, "Salvataggio automatico", false);
    cb2.on_toggled(|checked| println!("[CHECK] Autosave: {}", checked));

    let cb3 = XamlCheckBox::new(&frame, "Modalita scura", false);
    cb3.on_toggled(|checked| println!("[CHECK] Dark mode: {}", checked));

    // RadioButton in gruppo
    let lbl2 = XamlTextBlock::new(&frame, "Dimensione font:");
    lbl2.set_font_size(16.0);
    lbl2.set_bold(true);

    let _r1 = XamlRadioButton::new(&frame, "Piccolo", "font_size", false);
    let _r2 = XamlRadioButton::new(&frame, "Medio", "font_size", true);
    let _r3 = XamlRadioButton::new(&frame, "Grande", "font_size", false);

    // Cambia stato a runtime
    cb2.set_checked(true);
    println!("[SET] cb2.set_checked(true)");

    println!("3 checkbox + 3 radio. Clicca per verificare gli eventi.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
