//! Step 2 — TextBox / TextArea WinUI3/XAML
//!
//! Form con nome, password (placeholder), note multiline.
//! Echo live su stdout per ogni modifica.
//!
//! Run with: cargo run --example wx_step2_input --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlFrame, XamlTextBlock, XamlTextBox};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 2 — TextBox")
        .with_size(500, 400)
        .with_padding(20.0)
        .with_spacing(6.0)
        .build();

    println!("=== Step 2: TextBox ===");

    let _lbl_name = XamlTextBlock::new(&frame, "Nome:");
    let txt_name = XamlTextBox::new(&frame, "");
    txt_name.set_placeholder("Inserisci il tuo nome");
    txt_name.on_text_changed(|text| {
        println!("[TEXT] Nome: {}", text);
    });

    let _lbl_pass = XamlTextBlock::new(&frame, "Password:");
    let txt_pass = XamlTextBox::new(&frame, "");
    txt_pass.set_placeholder("Password (non sicura, solo demo)");
    txt_pass.on_text_changed(|text| {
        println!("[TEXT] Password: {} caratteri", text.len());
    });

    let _lbl_notes = XamlTextBlock::new(&frame, "Note:");
    let txt_notes = XamlTextBox::multiline(&frame, "Scrivi qui...\nSeconda riga.");
    txt_notes.on_text_changed(|text| {
        println!("[TEXT] Note: {} caratteri totali", text.len());
    });

    // Cambia valore a runtime
    txt_name.set_value("Marco");
    println!("[SET] txt_name.set_value(\"Marco\")");

    println!("3 campi input con echo live. Scrivi per verificare.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
