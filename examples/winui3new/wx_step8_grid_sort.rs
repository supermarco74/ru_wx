//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Step 8.3 — Grid sorting
//!
//! Sort programmatico per colonna (passo 8.3.2) chiamato prima del render:
//! le righe vengono riordinate davvero e l'header della colonna ordinata
//! mostra l'indicatore ↑/↓ (passo 8.3.6). `clear_sort()` (passo 8.3.4)
//! rimuove l'indicatore — l'ordine di inserimento non è memorizzato.
//!
//! Nota di backend: dopo `app.run()` il render closure lavora su una copia
//! delle `WidgetDesc` e gli handler sono `Send` — le mutazioni runtime del
//! desc non sono ancora propagate al render (come negli altri demo `wx_step*`,
//! i click loggano in console). Il sort efficace è quindi quello pre-render.
//! Passi non implementati: 8.3.1 (sort su click header), 8.3.3
//! (`sort_state()`), 8.3.5 (`set_column_sort_kind()`).
//!
//! Run with: cargo run --example wx_step8_grid_sort --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlButton, XamlFrame, XamlGrid, XamlTextBlock};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 8.3 — Grid sort")
        .with_size(760, 480)
        .with_padding(16.0)
        .with_spacing(12.0)
        .build();

    println!("=== Step 8.3: Grid sort ===");

    let _label = XamlTextBlock::new(&frame, "Dati caricati in ordine di inserimento:");

    let grid = XamlGrid::new(&frame);

    grid.add_column("Priorità", 80.0);
    grid.add_column("Attività", 320.0);
    grid.add_column("Stima (gg)", 100.0);

    grid.set_row_count(7);
    let rows = [
        ("B", "Ripulire warning clippy", "1"),
        ("A", "Backend Fase 8 avanzato", "8"),
        ("C", "Aggiornare README", "0.5"),
        ("A", "Dialoghi WinRT async", "5"),
        ("B", "Demo tray", "2"),
        ("C", "Screenshot doc", "1"),
        ("A", "Inspector XAML eventi", "4"),
    ];
    for (i, (prio, task, days)) in rows.iter().enumerate() {
        grid.set_cell_text(i, 0, prio);
        grid.set_cell_text(i, 1, task);
        grid.set_cell_text(i, 2, days);
    }

    grid.set_alternating_row_colors((255, 255, 255), (245, 245, 250));
    grid.set_selected_row(Some(0));

    // Passo 8.3.2 — sort programmatico pre-render (decrescente per
    // mostrare l'indicatore ↓ nell'header, passo 8.3.6)
    grid.sort_by_column(1, false);
    println!("[SORT] sort_by_column(1, false) → header \"Attività ↓\"");

    // Passi 8.3.2 / 8.3.4 — le altre chiamate, come negli altri demo,
    // loggano in console il proprio effetto
    let b_asc = XamlButton::new(&frame, "Sorta Attività ↑");
    b_asc.set_width(180.0);
    b_asc.on_click(|| {
        println!("[SORT] sort_by_column(1, true) → crescente (pre-render)");
    });

    let b_desc = XamlButton::new(&frame, "Sorta Attività ↓");
    b_desc.set_width(180.0);
    b_desc.on_click(|| {
        println!("[SORT] sort_by_column(1, false) → decrescente (pre-render)");
    });

    let b_clear = XamlButton::new(&frame, "Clear sort");
    b_clear.set_width(180.0);
    b_clear.on_click(|| {
        println!("[SORT] clear_sort() → solo indicatore rimosso (pre-render)");
    });

    let _note = XamlTextBlock::new(
        &frame,
        "Riordinate per Attività ↓ (pre-render). Clear sort toglie solo l'indicatore.",
    );

    println!("Grid riordinata per Attività decrescente. Verifica visiva + log click.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
