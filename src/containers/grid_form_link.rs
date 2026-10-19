//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Ponte **DataGrid ↔ AutoForm**: modifica di una riga tramite form.
//!
//! [`GridFormLink`] collega una [`DataGrid`] (vista tabellare su una
//! `DataTable`) a una [`AutoForm`] (editor dei campi della riga). Il
//! flusso tipico è:
//!
//! 1. [`GridFormLink::bind`] registra la callback di selezione: quando
//!    l'utente seleziona una riga nella griglia, i valori della riga
//!    **sorgente** (non quella filtrata/ordinata) vengono caricati nel
//!    form;
//! 2. l'utente modifica i campi del form;
//! 3. [`GridFormLink::apply`] riscrive la riga nella `DataTable` e
//!    invoca [`DataGrid::refresh`] per ridisegnare la vista.
//!
//! Il link non possiede i controlli: li condivide (`DataGrid` è
//! `Clone`, il form è dietro `Rc<RefCell<_>>` perché le callback di
//! selezione devono poterlo caricare).

use std::cell::RefCell;
use std::rc::Rc;

use crate::containers::auto_form::AutoForm;
use crate::containers::data_grid::DataGrid;
use crate::window::frame::Frame;

/// Collegamento bidirezionale tra una [`DataGrid`] e una [`AutoForm`].
///
/// Tiene traccia della riga sorgente attualmente in editing
/// ([`GridFormLink::editing_row`]): `apply` fallisce (ritornando
/// `false`) se nessuna riga è stata caricata.
#[derive(Clone)]
pub struct GridFormLink {
    grid: DataGrid,
    form: Rc<RefCell<AutoForm>>,
    editing: Rc<RefCell<Option<usize>>>,
}

impl GridFormLink {
    /// Crea il ponte tra una griglia e un form già costruiti sullo
    /// stesso schema di tabella.
    pub fn new(grid: DataGrid, form: AutoForm) -> Self {
        GridFormLink {
            grid,
            form: Rc::new(RefCell::new(form)),
            editing: Rc::new(RefCell::new(None)),
        }
    }

    /// La griglia collegata.
    pub fn grid(&self) -> &DataGrid {
        &self.grid
    }

    /// Il form collegato, dietro `Rc<RefCell<_>>` per l'uso nelle
    /// callback.
    pub fn form(&self) -> &Rc<RefCell<AutoForm>> {
        &self.form
    }

    /// Collega la selezione della griglia al caricamento del form:
    /// a ogni riga selezionata corrisponde un `load_row` con l'indice
    /// **sorgente**; azzerando la selezione il form resta com'è e lo
    /// stato di editing viene dimenticato.
    pub fn bind(&self, frame: &Frame) {
        let link = self.clone();
        self.grid
            .on_selection_changed(frame, move |source| match source {
                Some(row) => {
                    link.load_row(row);
                }
                None => *link.editing.borrow_mut() = None,
            });
    }

    /// Indice della riga sorgente attualmente in editing, se presente.
    pub fn editing_row(&self) -> Option<usize> {
        *self.editing.borrow()
    }

    /// Carica la riga sorgente `source_row` dalla tabella della
    /// griglia nel form. Ritorna `false` se la riga non esiste.
    pub fn load_row(&self, source_row: usize) -> bool {
        let values = self.grid.with_data(|t| t.row(source_row).cloned());
        match values {
            Some(row) => {
                self.form.borrow_mut().set_row(&row);
                *self.editing.borrow_mut() = Some(source_row);
                true
            }
            None => false,
        }
    }

    /// Riscrive la riga in editing con i valori correnti del form e
    /// ridisegna la griglia. Ritorna `false` se non c'è una riga in
    /// editing o se è sparita dalla tabella.
    pub fn apply(&self) -> bool {
        let source = match *self.editing.borrow() {
            Some(s) => s,
            None => return false,
        };
        let row = self.form.borrow().get_row();
        let updated = self.grid.update_data(|t| {
            if let Some(slot) = t.rows.get_mut(source) {
                // La riga dal form può essere più corta (colonne nascoste
                // restano nel form, ma per difesa riallineiamo).
                row_resize(&mut row.clone(), slot.len());
                *slot = row;
                true
            } else {
                false
            }
        });
        if updated {
            self.grid.refresh();
        }
        updated
    }

    // ── Persistenza su database ──────────────────────────────────────

    /// Salva la riga in editing nel database: `UPDATE ... WHERE pk = ?`
    /// con il valore **originale** della chiave (letto dalla tabella
    /// della griglia). Se l'`UPDATE` modifica almeno una riga, aggiorna
    /// anche la tabella locale e ridisegna la griglia.
    ///
    /// Errori: nessuna riga in editing, schema senza PK, oppure errore
    /// del provider.
    pub fn apply_to_provider(
        &self,
        provider: &mut dyn crate::data::provider::DataProvider,
    ) -> Result<u64, crate::data::error::DataError> {
        use crate::data::error::DataError;

        let source = self
            .editing
            .borrow()
            .ok_or_else(|| DataError::Unsupported("GridFormLink: nessuna riga in editing"))?;
        let schema = self.form.borrow().schema().clone();
        let pk_name = schema.primary_key.clone().ok_or_else(|| {
            DataError::Schema(format!("tabella {}: nessuna chiave primaria", schema.name))
        })?;
        let pk_idx = schema
            .column_index(&pk_name)
            .ok_or_else(|| DataError::Schema(format!("colonna PK {pk_name} assente")))?;
        let pk_original = self
            .grid
            .with_data(|t| t.cell(source, pk_idx).cloned())
            .ok_or_else(|| DataError::Schema(format!("riga {source} senza valore PK")))?;
        let row = self.form.borrow().get_row();
        let (sql, params) =
            crate::data::sql_gen::build_update(&schema, &row, &pk_original, &|i| {
                provider.quote_ident(i)
            })?;
        let n = provider.execute(&sql, &params)?;
        if n > 0 {
            self.apply();
        }
        Ok(n)
    }

    /// Inserisce il contenuto del form come **nuova riga**: `INSERT`
    /// (colonne auto-incremento escluse), poi aggiunge la riga alla
    /// tabella locale con l'id restituito da `last_insert_id` e
    /// ridisegna la griglia.
    pub fn insert_to_provider(
        &self,
        provider: &mut dyn crate::data::provider::DataProvider,
    ) -> Result<i64, crate::data::error::DataError> {
        let schema = self.form.borrow().schema().clone();
        let mut row = self.form.borrow().get_row();
        let (sql, params) =
            crate::data::sql_gen::build_insert(&schema, &row, &|i| provider.quote_ident(i))?;
        provider.execute(&sql, &params)?;
        let id = provider.last_insert_id()?;
        // Scrive l'id generato nella colonna PK della riga locale.
        if let Some(pk_name) = &schema.primary_key {
            if let Some(pk_idx) = schema.column_index(pk_name) {
                if pk_idx < row.len() {
                    row[pk_idx] = crate::data::value::DataValue::Int(id);
                }
            }
        }
        self.grid.update_data(|t| {
            row_resize(&mut row, t.columns.len());
            t.rows.push(row);
        });
        self.grid.refresh();
        Ok(id)
    }
}

/// Riporta `row` alla lunghezza `n`: tronca gli eccessi e riempie i
/// mancanti con `Null` (invariante della `DataTable`).
fn row_resize(row: &mut Vec<crate::data::value::DataValue>, n: usize) {
    row.resize(n, crate::data::value::DataValue::Null);
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    use crate::data::column::{DataColumn, DataFieldType};
    use crate::data::table::DataTable;
    use crate::data::value::DataValue;

    /// Tabella di prova: due colonne, due righe.
    fn tabella() -> DataTable {
        DataTable::from_rows(
            "prodotti",
            vec![
                DataColumn::new("id", DataFieldType::Integer),
                DataColumn::new("nome", DataFieldType::Text),
            ],
            vec![
                vec![DataValue::Int(1), DataValue::Text("Mela".to_string())],
                vec![DataValue::Int(2), DataValue::Text("Pera".to_string())],
            ],
        )
    }

    /// Costruisce griglia + form + link su un frame reale (i controlli
    /// Win32 richiedono un HWND valido).
    fn link_di_prova() -> (Frame, GridFormLink) {
        let frame = Frame::builder()
            .with_title("grid-form-link-test")
            .with_size(400, 300)
            .build();
        let grid = DataGrid::new(&frame);
        grid.set_data(&frame, tabella());
        let mut schema = crate::data::schema::TableSchema::new("prodotti");
        schema.columns = vec![
            DataColumn::new("id", DataFieldType::Integer),
            DataColumn::new("nome", DataFieldType::Text),
        ];
        let form = AutoForm::new(&frame, schema);
        (frame, GridFormLink::new(grid, form))
    }

    #[test]
    fn caricamento_riga_popola_il_form() {
        let (_frame, link) = link_di_prova();

        assert!(link.load_row(1));
        assert_eq!(link.editing_row(), Some(1));
        assert_eq!(
            link.form().borrow().get_value("nome"),
            Some(DataValue::Text("Pera".to_string()))
        );
        // Riga inesistente: nessun caricamento, editing invariato.
        assert!(!link.load_row(99));
        assert_eq!(link.editing_row(), Some(1));
    }

    #[test]
    fn apply_riscrive_la_riga_nella_tabella() {
        let (_frame, link) = link_di_prova();

        // Senza editing, apply fallisce.
        assert!(!link.apply());

        assert!(link.load_row(0));
        link.form()
            .borrow_mut()
            .set_value("nome", DataValue::Text("Mela verde".to_string()));
        assert!(link.apply());

        let nome = link.grid().with_data(|t| {
            t.cell_by_name(0, "nome")
                .cloned()
                .unwrap_or(DataValue::Null)
        });
        assert_eq!(nome, DataValue::Text("Mela verde".to_string()));
        // L'altra riga è intatta.
        let altra = link.grid().with_data(|t| {
            t.cell_by_name(1, "nome")
                .cloned()
                .unwrap_or(DataValue::Null)
        });
        assert_eq!(altra, DataValue::Text("Pera".to_string()));
    }

    #[test]
    fn bind_registra_la_callback_di_selezione() {
        // Verifica solo che il bind non fallisca: la selezione reale
        // arriva dal message loop Win32, non simulabile in unit test.
        let (frame, link) = link_di_prova();
        link.bind(&frame);
        assert_eq!(link.editing_row(), None);
    }

    // ── Persistenza (richiede SQLite) ────────────────────────────────

    /// Provider SQLite in-memory con la tabella `prodotti` e due righe.
    #[cfg(feature = "data-sqlite")]
    fn link_con_db() -> (
        Frame,
        GridFormLink,
        Box<dyn crate::data::provider::DataProvider>,
    ) {
        use crate::data::provider::DataProvider;

        let mut db = crate::data::connect("sqlite::memory:").expect("connect sqlite");
        db.execute(
            "CREATE TABLE prodotti (id INTEGER PRIMARY KEY AUTOINCREMENT, \
             nome TEXT NOT NULL, prezzo REAL)",
            &[],
        )
        .expect("create");
        db.execute(
            "INSERT INTO prodotti (nome, prezzo) VALUES ('Mela', 1.5), ('Pera', 2.0)",
            &[],
        )
        .expect("seed");

        let frame = Frame::builder()
            .with_title("grid-form-db-test")
            .with_size(400, 300)
            .build();
        let schema = db.table_schema("prodotti").expect("schema");
        let tabella = db.query_table("prodotti", None).expect("query");
        let grid = DataGrid::new(&frame);
        grid.set_data(&frame, tabella);
        let form = AutoForm::new(&frame, schema);
        (frame, GridFormLink::new(grid, form), db)
    }

    #[cfg(feature = "data-sqlite")]
    #[test]
    fn apply_to_provider_aggiorna_il_database() {
        use crate::data::provider::DataProvider;

        let (_frame, link, mut db) = link_con_db();
        assert!(link.load_row(0));
        link.form()
            .borrow_mut()
            .set_value("nome", DataValue::Text("Mela verde".to_string()));

        let n = link.apply_to_provider(&mut *db).expect("update");
        assert_eq!(n, 1);

        // Il DB è aggiornato...
        let t = db.query_table("prodotti", None).expect("rilettura");
        assert_eq!(
            t.cell_by_name(0, "nome"),
            Some(&DataValue::Text("Mela verde".to_string()))
        );
        // ...e anche la tabella locale della griglia.
        let locale = link.grid().with_data(|t| {
            t.cell_by_name(0, "nome")
                .cloned()
                .unwrap_or(DataValue::Null)
        });
        assert_eq!(locale, DataValue::Text("Mela verde".to_string()));
    }

    #[cfg(feature = "data-sqlite")]
    #[test]
    fn insert_to_provider_aggiunge_la_riga() {
        use crate::data::provider::DataProvider;

        let (_frame, link, mut db) = link_con_db();
        link.form()
            .borrow_mut()
            .set_value("nome", DataValue::Text("Banana".to_string()));
        link.form()
            .borrow_mut()
            .set_value("prezzo", DataValue::Real(0.9));

        let id = link.insert_to_provider(&mut *db).expect("insert");
        assert!(id >= 3, "id generato = {id}");

        let t = db.query_table("prodotti", None).expect("rilettura");
        assert_eq!(t.row_count(), 3);
        let locale_count = link.grid().with_data(|t| t.row_count());
        assert_eq!(locale_count, 3);
    }

    #[test]
    fn apply_to_provider_senza_editing_fallisce() {
        let (_frame, link) = link_di_prova();
        let mut mem = crate::data::memory::connect_memory();
        let err = link.apply_to_provider(&mut mem);
        assert!(err.is_err());
    }
}
