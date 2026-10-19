//! Step 9 — CheckListBox & Hyperlink WinUI3/XAML
//!
//! Lista con check-box per riga (omologa della `CheckListBox` di AppKit) e
//! hyperlink navigabile (omologo dell'`HyperlinkCtrl`, `HyperlinkButton`).
//!
//! Run with: cargo run --example wx_step9_checklist --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{
    XamlApp, XamlCheckListBox, XamlFrame, XamlHyperlink, XamlTextBlock,
};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 9 — CheckListBox & Hyperlink")
        .with_size(400, 380)
        .with_padding(16.0)
        .with_spacing(12.0)
        .build();

    println!("=== Step 9: CheckListBox + Hyperlink ===");

    let _label = XamlTextBlock::new(&frame, "Opzioni:");
    let checklist = XamlCheckListBox::new(&frame);
    checklist.append("Notifiche email");
    checklist.append("Aggiornamenti automatici");
    checklist.append("Tema scuro");
    checklist.append("Backup cloud");
    checklist.set_checked(1, true);
    checklist.set_checked(3, true);
    println!("[CHECK] 4 elementi, 2 spuntati");

    let _sub = XamlTextBlock::new(&frame, "Link:");
    let _link = XamlHyperlink::new(&frame, "Documentazione ru_wx", "https://example.com/docs");
    let _link2 = XamlHyperlink::new(&frame, "Sito ufficiale", "https://example.com");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
