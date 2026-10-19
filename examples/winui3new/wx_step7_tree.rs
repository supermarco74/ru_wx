//! Step 7 — TreeCtrl WinUI3/XAML
//!
//! Albero file system simulato con expand/collapse e selezione.
//!
//! Run with: cargo run --example wx_step7_tree --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlFrame, XamlTextBlock, XamlTreeCtrl};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 7 — TreeCtrl")
        .with_size(500, 400)
        .with_padding(16.0)
        .with_spacing(12.0)
        .build();

    println!("=== Step 7: TreeCtrl ===");

    let _label = XamlTextBlock::new(&frame, "Albero file system:");

    let tree = XamlTreeCtrl::new(&frame);

    // Root: Documents
    let docs = tree.add_root("Documents").expanded();
    let docs = docs.child("report.pdf").child("notes.txt").child_node(
        tree.add_root("Projects")
            .expanded()
            .child("ru_wx")
            .child("other"),
    );
    tree.add_node(docs);

    // Root: Downloads
    let downloads = tree.add_root("Downloads");
    let downloads = downloads.child("setup.exe").child("archive.zip");
    tree.add_node(downloads);

    // Root: Pictures
    let pics = tree.add_root("Pictures");
    let pics = pics.child("photo1.jpg").child("photo2.jpg");
    tree.add_node(pics);

    println!("Albero con 3 root, expand/collapse. Verifica visiva.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
