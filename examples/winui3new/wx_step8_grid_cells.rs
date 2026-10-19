//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Step 8.2 — Grid cell types avanzati
//!
//! Colonne con Badge colorato, barra Progress, Bool (checkbox) e Text.
//! I tipi previsti dal piano 8.2.1-8.2.10 ma non presenti in
//! `GridCellDesc` (Image, Bar, MultiLine, Priority, Number, Date)
//! restano da implementare nel desc.
//!
//! Run with: cargo run --example wx_step8_grid_cells --features winui3_xaml

#![cfg(target_os = "windows")]

use ru_wx::platform::winui3_xaml::element_bridge::{GridBadgeKind, GridCellDesc};
use ru_wx::platform::winui3_xaml::{XamlApp, XamlFrame, XamlGrid, XamlTextBlock};

fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 8.2 — Grid cell types")
        .with_size(800, 460)
        .with_padding(16.0)
        .with_spacing(12.0)
        .build();

    println!("=== Step 8.2: Grid cell types ===");

    let _label = XamlTextBlock::new(&frame, "Celle Badge, Progress, Bool e Text:");

    let grid = XamlGrid::new(&frame);

    grid.add_column("Servizio", 160.0);
    grid.add_column("Stato", 110.0); // Badge
    grid.add_column("Completamento", 160.0); // Progress
    grid.add_column("Attivo", 70.0); // Bool
    grid.add_column("Descrizione", 220.0); // Text

    grid.set_row_count(6);
    let services = [
        (
            "build",
            GridBadgeKind::Good,
            100,
            true,
            "tutti i target compilano",
        ),
        ("tests", GridBadgeKind::Good, 100, true, "suite verde"),
        (
            "clippy",
            GridBadgeKind::Warn,
            75,
            true,
            "4 warning preesistenti",
        ),
        ("docs", GridBadgeKind::Info, 90, true, "cargo doc pulito"),
        (
            "mac",
            GridBadgeKind::Bad,
            0,
            false,
            "verifica saltata su richiesta",
        ),
        (
            "tray",
            GridBadgeKind::Warn,
            50,
            false,
            "API non in winui3_xaml",
        ),
    ];

    for (row, (name, kind, progress, active, note)) in services.iter().enumerate() {
        grid.set_cell_text(row, 0, name);
        // Passo 8.2.5 — badge colorato
        grid.set_cell(
            row,
            1,
            GridCellDesc::Badge {
                text: match kind {
                    GridBadgeKind::Good => "OK".to_string(),
                    GridBadgeKind::Warn => "Parziale".to_string(),
                    GridBadgeKind::Bad => "Mancante".to_string(),
                    GridBadgeKind::Info => "Info".to_string(),
                },
                kind: *kind,
            },
        );
        // Passo 8.2.3 — barra progresso
        grid.set_cell(
            row,
            2,
            GridCellDesc::Progress {
                value: *progress,
                max: 100,
            },
        );
        // Passo 8.2.10 — checkbox in cella
        grid.set_cell(row, 3, GridCellDesc::Bool(*active));
        grid.set_cell_text(row, 4, note);
    }

    grid.set_alternating_row_colors((255, 255, 255), (245, 245, 250));
    grid.set_selected_row(Some(0));

    println!("6 righe con Badge (Good/Warn/Bad/Info), Progress, Bool, Text.");

    match app.run(frame) {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("Errore: {:?}", e),
    }
}
