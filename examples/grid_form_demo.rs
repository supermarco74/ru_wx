//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Demo: integrazione **DataGrid ↔ AutoForm ↔ database** (SQLite
//! in-memory). Il giro completo:
//!
//! 1. la `DataGrid` mostra la tabella `prodotti` letta dal DB;
//! 2. selezionando una riga, `GridFormLink` la carica nella `AutoForm`
//!    (generata dallo schema del DB, con combo per la FK
//!    `categoria_id`);
//! 3. **Applica modifiche** valida il form (`AutoForm::validate`) e
//!    salva con `UPDATE ... WHERE id = ?` (`apply_to_provider`);
//! 4. **Nuovo prodotto** inserisce il contenuto del form con `INSERT`
//!    (`insert_to_provider`) e aggiunge la riga alla griglia.
//!
//! Gli errori di validazione e di database compaiono nella barra di
//! stato in basso.
//!
//! Run with:
//! ```bash
//! cargo run --example grid_form_demo --features data-sqlite
//! ```

#![windows_subsystem = "windows"]

use std::cell::RefCell;
use std::rc::Rc;

use ru_wx::{
    connect, App, AutoForm, Button, DataGrid, DataProvider, DataTable, DataValue, ForeignKey,
    Frame, GridFormLink, ImageList, StaticText,
};

/// Database SQLite in-memory con `categorie` + `prodotti` (FK).
fn setup_db() -> Box<dyn DataProvider> {
    let mut db = connect("sqlite::memory:").expect("connect sqlite");
    db.execute(
        "CREATE TABLE categorie (id INTEGER PRIMARY KEY, nome TEXT NOT NULL)",
        &[],
    )
    .expect("create categorie");
    db.execute(
        "INSERT INTO categorie (nome) VALUES ('Elettronica'), ('Casa e ufficio')",
        &[],
    )
    .expect("seed categorie");
    db.execute(
        "CREATE TABLE prodotti (\
             id INTEGER PRIMARY KEY AUTOINCREMENT, \
             nome TEXT NOT NULL, \
             prezzo REAL, \
             attivo INTEGER, \
             categoria_id INTEGER REFERENCES categorie(id))",
        &[],
    )
    .expect("create prodotti");
    db.execute(
        "INSERT INTO prodotti (nome, prezzo, attivo, categoria_id) VALUES \
         ('Monitor 27 pollici', 219.9, 1, 1), \
         ('Tastiera meccanica', 89.5, 1, 1), \
         ('Sedia ergonomica', 349.0, 0, 2)",
        &[],
    )
    .expect("seed prodotti");
    db
}

/// Lookup FK: legge le categorie dal DB ad ogni apertura del form.
fn lookup_categorie(
    db: Rc<RefCell<Box<dyn DataProvider>>>,
) -> impl Fn(&ForeignKey) -> Vec<(String, String)> {
    move |_fk| {
        let mut db = db.borrow_mut();
        db.query("SELECT id, nome FROM categorie ORDER BY nome", &[])
            .map(|t: DataTable| {
                t.rows
                    .iter()
                    .map(|r| {
                        (
                            r.first().map(|v| v.to_display_string()).unwrap_or_default(),
                            r.get(1).map(|v| v.to_display_string()).unwrap_or_default(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Posiziona griglia, form, pulsanti e barra di stato.
fn layout_all(
    grid: &DataGrid,
    form: &Rc<RefCell<AutoForm>>,
    apply_btn: &Button,
    new_btn: &Button,
    status: &StaticText,
    w: u32,
    h: u32,
) {
    let half = (w / 2) as i32;
    let height = h as i32;

    let g = grid.as_widget_ref();
    {
        let mut g = g.borrow_mut();
        g.set_position(8, 8);
        g.set_size((half - 16).max(50) as u32, (height - 48).max(50) as u32);
    }

    let fx = half + 8;
    let fw = (w as i32 - half - 16).max(120);
    let form_h = (height - 100).max(100);
    form.borrow_mut().layout(fx, 8, fw, form_h);

    let btn_w = ((fw - 8) / 2).max(60);
    let b1 = apply_btn.as_widget_ref();
    {
        let mut b = b1.borrow_mut();
        b.set_position(fx, height - 84);
        b.set_size(btn_w as u32, 28);
    }
    let b2 = new_btn.as_widget_ref();
    {
        let mut b = b2.borrow_mut();
        b.set_position(fx + btn_w + 8, height - 84);
        b.set_size(btn_w as u32, 28);
    }

    let s = status.as_widget_ref();
    {
        let mut s = s.borrow_mut();
        s.set_position(8, height - 40);
        s.set_size((w as i32 - 16).max(50) as u32, 24);
    }
}

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("ru_wx — DataGrid + AutoForm + SQLite")
        .with_size(920, 560)
        .build();

    let db = Rc::new(RefCell::new(setup_db()));

    // Schema e dati dal DB; il form nasce dallo schema reale.
    let schema = db
        .borrow_mut()
        .table_schema("prodotti")
        .expect("schema prodotti");
    let tabella = db
        .borrow_mut()
        .query_table("prodotti", None)
        .expect("dati prodotti");

    let grid = DataGrid::new(&frame);
    grid.set_data(&frame, tabella);
    // Sort iniziale per nome: mostra la freccia nativa sull'header.
    grid.set_sort(Some(1), true);

    // Icone di riga (SVG Lucide embedded → ImageList nativa 16×16):
    // l'indice immagine vive nel modello vista, quindi segue la riga
    // attraverso sort e filtri (`LVIF_IMAGE` in modalità virtuale).
    let icone = ImageList::new(16, 16);
    let idx_cpu = icone.add_svg_bytes(include_bytes!("../assets/icons/lucide/cpu.svg"));
    let idx_zap = icone.add_svg_bytes(include_bytes!("../assets/icons/lucide/zap.svg"));
    let idx_pkg = icone.add_svg_bytes(include_bytes!("../assets/icons/lucide/package.svg"));
    grid.set_image_list(icone);
    // Righe sorgente: 0=Monitor, 1=Tastiera, 2=Sedia (ordine INSERT).
    if let Some(i) = idx_cpu {
        grid.set_row_image(0, Some(i));
    }
    if let Some(i) = idx_zap {
        grid.set_row_image(1, Some(i));
    }
    if let Some(i) = idx_pkg {
        grid.set_row_image(2, Some(i));
    }

    let form = AutoForm::with_fk_lookup(&frame, schema.clone(), lookup_categorie(db.clone()));

    // Ponte: selezione riga → form; pulsante → riscrittura riga.
    let link = GridFormLink::new(grid.clone(), form);
    link.bind(&frame);

    let status = StaticText::new(&frame, "Seleziona una riga per modificarla.");

    // Checkbox native (LVS_EX_CHECKBOXES) con conteggio nella status bar.
    grid.set_checkboxes(true);
    {
        let grid_inner = grid.clone();
        let status = status.clone();
        grid.on_check_changed(move |_row, _checked| {
            let n = grid_inner.checked_rows().len();
            status.set_label(&format!("{n} righe spuntate."));
        });
    }

    // Edit in-place: doppio click su una cella → EDIT nativo; al commit
    // la riga è subito persistita con UPDATE ... WHERE id = ?.
    grid.set_editable(true);
    {
        let db = db.clone();
        let grid_inner = grid.clone();
        let status = status.clone();
        let schema_update = schema.clone();
        grid.on_cell_edited(move |source_row, _col, _value| {
            let row = grid_inner.with_data(|t| t.row(source_row).cloned());
            let Some(row) = row else { return };
            let pk_idx = schema_update
                .primary_key
                .as_ref()
                .and_then(|pk| schema_update.column_index(pk));
            let Some(pk_idx) = pk_idx else { return };
            let pk_orig = row.get(pk_idx).cloned().unwrap_or(DataValue::Null);
            let mut db = db.borrow_mut();
            let result =
                ru_wx::build_update(&schema_update, &row, &pk_orig, &|i| db.quote_ident(i))
                    .and_then(|(sql, params)| db.execute(&sql, &params));
            match result {
                Ok(n) => status.set_label(&format!("Cella salvata nel DB ({n} riga aggiornata).")),
                Err(e) => status.set_label(&format!("Errore DB: {e}")),
            }
        });
    }

    let apply_btn = Button::new(&frame, "Applica modifiche");
    {
        let link = link.clone();
        let db = db.clone();
        let status = status.clone();
        apply_btn.on_click(&frame, move || {
            // Validazione prima del salvataggio.
            let errors = link.form().borrow().validate();
            if !errors.is_empty() {
                status.set_label(&format!("Errore: {}", errors.join("; ")));
                return;
            }
            let mut db = db.borrow_mut();
            match link.apply_to_provider(&mut **db) {
                Ok(n) => status.set_label(&format!("Salvato ({n} riga aggiornata).")),
                Err(e) => status.set_label(&format!("Errore DB: {e}")),
            }
        });
    }

    let new_btn = Button::new(&frame, "Nuovo prodotto");
    {
        let link = link.clone();
        let db = db.clone();
        let status = status.clone();
        new_btn.on_click(&frame, move || {
            let errors = link.form().borrow().validate();
            if !errors.is_empty() {
                status.set_label(&format!("Errore: {}", errors.join("; ")));
                return;
            }
            let mut db = db.borrow_mut();
            match link.insert_to_provider(&mut **db) {
                Ok(id) => status.set_label(&format!("Inserito prodotto #{id}.")),
                Err(e) => status.set_label(&format!("Errore DB: {e}")),
            }
        });
    }

    // Layout manuale (il form usa un FlexGridSizer interno non
    // annidabile nei BoxSizer) + riposizionamento al resize.
    let (w, h) = frame.client_size();
    layout_all(&grid, link.form(), &apply_btn, &new_btn, &status, w, h);
    {
        let grid = grid.clone();
        let form = link.form().clone();
        let apply_btn = apply_btn.clone();
        let new_btn = new_btn.clone();
        let status = status.clone();
        frame.on_resize(move |w, h| layout_all(&grid, &form, &apply_btn, &new_btn, &status, w, h));
    }

    app.run(frame);
}
