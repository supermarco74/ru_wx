//! Step 4 — ToolBar WinUI3/XAML
//!
//! Toolbar con 5 bottoni, separatori, un toggle.
//!
//! Run with: cargo run --example wx_step4_toolbar --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlFrame, XamlTextBlock, XamlToolBar};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 4 — ToolBar")
        .with_size(600, 300)
        .with_padding(16.0)
        .with_spacing(12.0)
        .build();

    println!("=== Step 4: ToolBar ===");

    let toolbar = XamlToolBar::new(&frame);
    toolbar.add_button("New");
    toolbar.add_button("Open");
    toolbar.add_button("Save");
    toolbar.add_separator();
    toolbar.add_toggle("Bold");
    toolbar.add_separator();
    toolbar.add_button("Delete");

    let _content = XamlTextBlock::new(
        &frame,
        "ToolBar con 5 bottoni + 1 toggle. Clicca per verificare.",
    );

    println!("ToolBar con bottoni. Clicca per verificare.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
