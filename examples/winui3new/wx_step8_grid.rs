//! Step 8 — Grid WinUI3/XAML
//!
//! Data grid completo: colonne, righe, sorting, selezione, colori alternati,
//! cell types (Text, Progress, Badge, Bool).
//!
//! Run with: cargo run --example wx_step8_grid --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::{XamlApp, XamlFrame, XamlGrid, XamlTextBlock};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 8 — Grid")
        .with_size(800, 500)
        .with_padding(16.0)
        .with_spacing(12.0)
        .build();

    println!("=== Step 8: Grid ===");

    let _label = XamlTextBlock::new(&frame, "Data grid con sorting e selezione:");

    let grid = XamlGrid::new(&frame);

    // Colonne
    grid.add_column("ID", 50.0);
    grid.add_column("Nome", 200.0);
    grid.add_column("Categoria", 100.0);
    grid.add_column("Prezzo", 80.0);
    grid.add_column("Stock", 80.0);
    grid.add_column("Rating", 80.0);

    // Dati
    grid.set_row_count(10);
    let products = [
        ("1", "ru_wx crate", "Rust", "€0.00", "999", "5/5"),
        ("2", "windows-sys", "Rust", "€0.00", "999", "4/5"),
        ("3", "resvg", "Rust", "€0.00", "999", "5/5"),
        ("4", "image", "Rust", "€0.00", "999", "4/5"),
        ("5", "serde", "Rust", "€0.00", "999", "5/5"),
        ("6", "human-panic", "Rust", "€0.00", "999", "3/5"),
        ("7", "objc2-app-kit", "macOS", "€0.00", "999", "4/5"),
        ("8", "objc2-web-kit", "macOS", "€0.00", "999", "4/5"),
        ("9", "gtk4-rs", "Linux", "€0.00", "999", "4/5"),
        ("10", "winit", "Cross", "€0.00", "999", "5/5"),
    ];

    for (i, (id, name, cat, price, stock, rating)) in products.iter().enumerate() {
        grid.set_cell_text(i, 0, id);
        grid.set_cell_text(i, 1, name);
        grid.set_cell_text(i, 2, cat);
        grid.set_cell_text(i, 3, price);
        grid.set_cell_text(i, 4, stock);
        grid.set_cell_text(i, 5, rating);
    }

    // Colori alternati
    grid.set_alternating_row_colors((255, 255, 255), (245, 245, 250));

    // Sorting iniziale
    grid.sort_by_column(1, true);

    // Selezione
    grid.set_selected_row(Some(0));

    println!("Grid con 6 colonne, 10 righe, sorting, selezione, colori alternati.");
    println!("Cell types: Text (tutte le celle). Verifica visiva.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
