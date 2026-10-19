//! Step 5 — Dialogs WinUI3/XAML
//!
//! 4 bottoni che aprono dialoghi diversi: MessageDialog, ColorPicker,
//! FileDialog (stub), FontDialog (stub), DirDialog (stub).
//!
//! Run with: cargo run --example wx_step5_msgbox --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{
    XamlApp, XamlButton, XamlColorDialog, XamlFileDialog, XamlFrame, XamlMessageDialog,
    XamlTextBlock,
};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 5 — Dialogs")
        .with_size(600, 400)
        .with_padding(16.0)
        .with_spacing(12.0)
        .build();

    println!("=== Step 5: Dialogs ===");

    let _label = XamlTextBlock::new(&frame, "Clicca i bottoni per aprire i dialoghi:");

    // MessageDialog
    let btn_msg = XamlButton::new(&frame, "Message Dialog");
    btn_msg.on_click(|| {
        println!("[DIALOG] MessageDialog aperto");
    });
    let _msg_dlg = XamlMessageDialog::new(
        &frame,
        "Conferma",
        "Sei sicuro di voler procedere?",
        "Si",
        "No",
        "Annulla",
    );

    // ColorPicker
    let btn_color = XamlButton::new(&frame, "Color Picker");
    btn_color.on_click(|| {
        println!("[DIALOG] ColorPicker aperto");
    });
    let _color_dlg = XamlColorDialog::new(&frame, 79, 70, 229);

    // FileDialog (stub)
    let btn_file = XamlButton::new(&frame, "File Dialog (stub)");
    btn_file.on_click(|| {
        println!("[DIALOG] FileDialog — stub, non implementato");
        let _ = XamlFileDialog::open(&XamlFrame::builder().build(), "Open", "*.txt");
    });

    // FontDialog (stub)
    let btn_font = XamlButton::new(&frame, "Font Dialog (stub)");
    btn_font.on_click(|| {
        println!("[DIALOG] FontDialog — stub, non implementato");
    });

    // DirDialog (stub)
    let btn_dir = XamlButton::new(&frame, "Dir Dialog (stub)");
    btn_dir.on_click(|| {
        println!("[DIALOG] DirDialog — stub, non implementato");
    });

    println!("5 dialoghi (2 funzionanti, 3 stub). Clicca per verificare.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
