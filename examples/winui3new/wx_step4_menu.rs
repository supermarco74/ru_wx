//! Step 4 — MenuBar WinUI3/XAML
//!
//! Menu completo: File (New, Open, Save, Exit), Edit (Cut, Copy, Paste),
//! Help (About). Verifica: ogni voce logga su stdout.
//!
//! Run with: cargo run --example wx_step4_menu --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlFrame, XamlMenu, XamlMenuBar, XamlTextBlock};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 4 — MenuBar")
        .with_size(600, 400)
        .with_padding(16.0)
        .with_spacing(12.0)
        .build();

    println!("=== Step 4: MenuBar ===");

    let menubar = XamlMenuBar::new(&frame);

    // File menu
    let mut file_menu = XamlMenu::new("File");
    file_menu.add_item("New");
    file_menu.add_item("Open");
    file_menu.add_item("Save");
    file_menu.add_separator();
    file_menu.add_item("Exit");
    menubar.add_menu(file_menu);

    // Edit menu
    let mut edit_menu = XamlMenu::new("Edit");
    edit_menu.add_item("Cut");
    edit_menu.add_item("Copy");
    edit_menu.add_item("Paste");
    edit_menu.add_separator();
    edit_menu.add_item_disabled("Undo (non disponibile)");
    menubar.add_menu(edit_menu);

    // Help menu con submenu
    let mut help_menu = XamlMenu::new("Help");
    help_menu.add_item("About");
    let mut theme_menu = XamlMenu::new("Theme");
    theme_menu.add_item("Light");
    theme_menu.add_item("Dark");
    theme_menu.add_item("System");
    help_menu.add_submenu("Preferences", theme_menu);
    menubar.add_menu(help_menu);

    let _content = XamlTextBlock::new(&frame, "MenuBar con 3 menu. Apri e clicca le voci.");

    println!("MenuBar con File, Edit, Help. Clicca le voci per verificare.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
