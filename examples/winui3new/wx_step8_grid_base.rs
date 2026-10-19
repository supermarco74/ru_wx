//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Step 8.1 — Grid base: visualizzazione dati
//!
//! 5 colonne × 20 righe con celle Text e Empty, selezione riga singola
//! e colori di riga alternati. Copre i passi 8.1.1-8.1.6 del piano.
//!
//! Run with: cargo run --example wx_step8_grid_base --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::element_bridge::GridCellDesc;
use ru_wx::platform::winui3_xaml::{XamlApp, XamlFrame, XamlGrid, XamlTextBlock};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 8.1 — Grid base")
        .with_size(800, 500)
        .with_padding(16.0)
        .with_spacing(12.0)
        .build();

    println!("=== Step 8.1: Grid base ===");

    let _label = XamlTextBlock::new(&frame, "Tabella 5 colonne × 20 righe (celle Text + Empty):");

    let grid = XamlGrid::new(&frame);

    // Passo 8.1.2 — colonne con header e larghezza
    grid.add_column("ID", 50.0);
    grid.add_column("Nome", 220.0);
    grid.add_column("Linguaggio", 110.0);
    grid.add_column("Versione", 90.0);
    grid.add_column("Note", 180.0);

    // Passi 8.1.3-8.1.5 — righe virtualizzate + rendering Text/Empty
    grid.set_row_count(20);
    for row in 0..20 {
        grid.set_cell_text(row, 0, &(row + 1).to_string());
        grid.set_cell_text(row, 1, &format!("modulo_{}", row + 1));
        if row % 3 == 0 {
            grid.set_cell_text(row, 2, "Rust");
        } else {
            // Passo 8.1.5 — Cell::Empty (cella vuota)
            grid.set_cell(row, 2, GridCellDesc::Empty);
        }
        grid.set_cell_text(row, 3, &format!("{}.{}", row / 5, row % 5));
        if row % 4 == 0 {
            // Passo 8.1.5 — più celle Empty in colonna Note
            grid.set_cell(row, 4, GridCellDesc::Empty);
        } else {
            grid.set_cell_text(row, 4, "nessuna nota");
        }
    }

    // Passo 8.1.6 — selezione riga singola + colori alternati
    grid.set_selected_row(Some(0));
    grid.set_alternating_row_colors((255, 255, 255), (245, 245, 250));

    println!("20 righe, celle Text/Empty, selezione riga 1, alternating colors.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
