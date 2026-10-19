//! Step 2 — Button WinUI3/XAML
//!
//! Dimostra la creazione di bottoni XAML nativi con click handler.
//! 3 bottoni: incrementa, decrementa, reset — con contatore visualizzato.
//!
//! Verifica: i bottoni rispondono al click, il contatore si aggiorna.
//!
//! Run with: cargo run --example wx_step2_button --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlButton, XamlFrame, XamlTextBlock};

fn main() {
    let app = XamlApp::new();

    let frame = XamlFrame::builder()
        .with_title("Step 2 — Button")
        .with_size(500, 300)
        .with_padding(20.0)
        .with_spacing(12.0)
        .build();

    println!("=== Step 2: Button ===");

    // Contatore condiviso (semplice, per demo)
    let counter = std::sync::Arc::new(std::sync::Mutex::new(0_i32));

    // Label che mostra il contatore
    let label = XamlTextBlock::new(&frame, "Count: 0");
    label.set_font_size(24.0);
    label.set_bold(true);

    // Bottone incrementa
    let btn_inc = XamlButton::new(&frame, "Increment");
    btn_inc.set_width(180.0);
    {
        let counter = counter.clone();
        btn_inc.on_click(move || {
            let mut c = counter.lock().unwrap();
            *c += 1;
            println!("[CLICK] Increment → count = {}", *c);
        });
    }

    // Bottone decrementa
    let btn_dec = XamlButton::new(&frame, "Decrement");
    btn_dec.set_width(180.0);
    {
        let counter = counter.clone();
        btn_dec.on_click(move || {
            let mut c = counter.lock().unwrap();
            *c -= 1;
            println!("[CLICK] Decrement → count = {}", *c);
        });
    }

    // Bottone reset
    let btn_reset = XamlButton::new(&frame, "Reset");
    btn_reset.set_width(180.0);
    {
        let counter = counter.clone();
        btn_reset.on_click(move || {
            let mut c = counter.lock().unwrap();
            *c = 0;
            println!("[CLICK] Reset → count = 0");
        });
    }

    // Bottone disabilitato (non deve rispondere)
    let btn_disabled = XamlButton::new(&frame, "Disabled");
    btn_disabled.set_width(180.0);
    btn_disabled.set_enabled(false);

    println!("3 bottoni attivi + 1 disabilitato. Clicca per verificare gli eventi.");

    match app.run(frame) {
        Ok(_) => println!("Applicazione terminata."),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
