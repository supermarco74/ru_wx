//! Step 7 — ListBox WinUI3/XAML
//!
//! Lista con add/remove, selezione visualizzata su stdout.
//!
//! Run with: cargo run --example wx_step7_listbox --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlFrame, XamlListBox, XamlTextBlock};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 7 — ListBox")
        .with_size(400, 350)
        .with_padding(16.0)
        .with_spacing(12.0)
        .build();

    println!("=== Step 7: ListBox ===");

    let _label = XamlTextBlock::new(&frame, "Elementi:");

    let listbox = XamlListBox::new(&frame);
    listbox.append("Elemento 1");
    listbox.append("Elemento 2");
    listbox.append("Elemento 3");
    listbox.append("Elemento 4");
    listbox.append("Elemento 5");
    listbox.set_selected(0);

    // Aggiungi altri elementi a runtime
    listbox.append("Elemento 6 (aggiunto dopo)");
    println!("[ADD] 6 elementi in lista");

    println!("ListBox con 6 elementi. Seleziona per verificare.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
