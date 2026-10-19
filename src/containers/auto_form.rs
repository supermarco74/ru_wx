//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! `AutoForm`: form modali generate automaticamente da uno [`TableSchema`].
//!
//! Per ogni colonna lo schema genera una riga "etichetta | campo" in un
//! [`FlexGridSizer`] a due colonne (la seconda cresce), scegliendo il
//! controllo in base al tipo logico:
//!
//! | `DataFieldType` | Controllo                          |
//! |-----------------|------------------------------------|
//! | `Text`          | [`TextCtrl`]                       |
//! | `Integer`       | [`SpinCtrl`]                       |
//! | `Real`          | [`SpinCtrlDouble`]                 |
//! | `Boolean`       | [`CheckBox`]                       |
//! | `Date`/`DateTime` | [`DatePickerCtrl`]               |
//! | `Blob`          | [`StaticText`] non editabile       |
//!
//! Le colonne con una [`ForeignKey`] e un lookup fornito diventano
//! [`ComboBox`] con le voci della tabella referenziata (chiave +
//! etichetta leggibile); senza lookup resta un semplice `TextCtrl`.
//! Le colonne auto-incrementanti vengono disabilitate.
//!
//! # Layout
//!
//! `FlexGridSizer` non è annidabile in un [`BoxSizer`](crate::containers::sizer::BoxSizer):
//! il form va driveato manualmente, richiamando [`AutoForm::layout`] dal
//! gestore di ridimensionamento del contenitore:
//!
//! ```text
//! fn on_size(&mut self) {
//!     let (w, h) = self.frame.client_size();
//!     self.form.layout(8, 8, (w as i32) - 16, (h as i32) - 16);
//! }
//! ```
//!
//! # Valori e layout
//!
//! - [`AutoForm::set_row`] / [`AutoForm::get_row`] legano i campi a
//!   [`DataValue`] nell'ordine delle colonne dello schema;
//! - [`AutoForm::save_layout`] / [`AutoForm::load_layout`] serializzano
//!   in JSON la visibilità delle colonne (via `serde_json`).

use std::cell::RefCell;
use std::rc::Rc;

use serde::{Deserialize, Serialize};

use crate::containers::grid_sizer::FlexGridSizer;
use crate::controls::checkbox::CheckBox;
use crate::controls::combo_box::ComboBox;
use crate::controls::date_picker_ctrl::{Date, DateFormat, DatePickerCtrl};
use crate::controls::spin_ctrl::SpinCtrl;
use crate::controls::spin_ctrl_double::SpinCtrlDouble;
use crate::controls::static_text::StaticText;
use crate::controls::text_ctrl::TextCtrl;
use crate::core::widget::{WidgetRef, Window};
use crate::data::column::{DataColumn, DataFieldType};
use crate::data::foreign_key::ForeignKey;
use crate::data::schema::TableSchema;
use crate::data::value::DataValue;

/// Lookup FK: per ogni colonna con chiave esterna restituisce le coppie
/// `(chiave, etichetta)` da inserire nella [`ComboBox`].
pub type FkLookup = Rc<dyn Fn(&ForeignKey) -> Vec<(String, String)>>;

/// Tipo di controllo effettivamente generato per una colonna.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlKind {
    /// Campo testo libero ([`TextCtrl`]).
    Text,
    /// Campo intero ([`SpinCtrl`]).
    Integer,
    /// Campo decimale ([`SpinCtrlDouble`]).
    Real,
    /// Casella di spunta ([`CheckBox`]).
    Boolean,
    /// Selettore data ([`DatePickerCtrl`]).
    Date,
    /// Combo per chiave esterna ([`ComboBox`]).
    Combo,
    /// Rappresentazione binaria non editabile ([`StaticText`]).
    Blob,
}

// ── Controllo di un campo ─────────────────────────────────────────────

/// Controllo effettivamente generato per una colonna.
enum FieldControl {
    Text(TextCtrl),
    Int(SpinCtrl),
    Real(SpinCtrlDouble),
    Bool(CheckBox),
    Date(DatePickerCtrl),
    /// Combo FK: `keys` tiene le chiavi in ordine di append, così la
    /// selezione visualizzata (etichetta) resta mappata alla chiave.
    Combo {
        combo: ComboBox,
        keys: Rc<RefCell<Vec<String>>>,
    },
    /// Blob: etichetta non editabile + valore originale conservato
    /// (i byte non sono rappresentabili in un controllo di testo).
    Blob {
        view: StaticText,
        stored: Option<DataValue>,
    },
}

impl FieldControl {
    /// Riferimento type-erased del controllo (per il sizer e per
    /// `set_visible`/`set_enabled`).
    fn widget(&self) -> WidgetRef {
        match self {
            FieldControl::Text(c) => c.as_widget_ref(),
            FieldControl::Int(c) => c.as_widget_ref(),
            FieldControl::Real(c) => c.as_widget_ref(),
            FieldControl::Bool(c) => c.as_widget_ref(),
            FieldControl::Date(c) => c.as_widget_ref(),
            FieldControl::Combo { combo, .. } => combo.as_widget_ref(),
            FieldControl::Blob { view, .. } => view.as_widget_ref(),
        }
    }

    /// Tipo di controllo generato.
    fn kind(&self) -> ControlKind {
        match self {
            FieldControl::Text(_) => ControlKind::Text,
            FieldControl::Int(_) => ControlKind::Integer,
            FieldControl::Real(_) => ControlKind::Real,
            FieldControl::Bool(_) => ControlKind::Boolean,
            FieldControl::Date(_) => ControlKind::Date,
            FieldControl::Combo { .. } => ControlKind::Combo,
            FieldControl::Blob { .. } => ControlKind::Blob,
        }
    }

    /// Id nativo del controllo (per `register_command_handler`);
    /// `None` per i campi senza eventi (blob).
    fn id(&self) -> Option<u16> {
        match self {
            FieldControl::Text(c) => Some(c.id()),
            FieldControl::Int(c) => Some(c.id()),
            FieldControl::Real(c) => Some(c.id()),
            FieldControl::Bool(c) => Some(c.id()),
            FieldControl::Date(c) => Some(c.id()),
            FieldControl::Combo { combo, .. } => Some(combo.id()),
            FieldControl::Blob { .. } => None,
        }
    }

    /// Legge il valore corrente del controllo.
    fn get(&self, column: &DataColumn) -> DataValue {
        match self {
            FieldControl::Text(c) => {
                let v = c.get_value();
                if v.is_empty() && column.nullable {
                    DataValue::Null
                } else {
                    DataValue::Text(v)
                }
            }
            FieldControl::Int(c) => DataValue::Int(c.get_value() as i64),
            FieldControl::Real(c) => DataValue::Real(c.get_value()),
            FieldControl::Bool(c) => DataValue::Bool(c.is_checked()),
            FieldControl::Date(c) => match c.get_value() {
                Some(d) => DataValue::Text(format_iso_date(d)),
                None => DataValue::Null,
            },
            FieldControl::Combo { combo, keys } => match combo.get_selection() {
                Some(i) => keys
                    .borrow()
                    .get(i)
                    .cloned()
                    .map(DataValue::Text)
                    .unwrap_or(DataValue::Null),
                None => {
                    let t = combo.get_value();
                    if t.is_empty() {
                        DataValue::Null
                    } else {
                        DataValue::Text(t)
                    }
                }
            },
            FieldControl::Blob { stored, .. } => stored.clone().unwrap_or(DataValue::Null),
        }
    }

    /// Scrive un valore nel controllo.
    fn set(&mut self, column: &DataColumn, value: DataValue) {
        match self {
            FieldControl::Text(c) => match value {
                DataValue::Null => c.set_value(""),
                other => c.set_value(&other.to_display_string()),
            },
            FieldControl::Int(c) => match value {
                DataValue::Null => c.set_value(0),
                other => {
                    if let Some(i) = other.as_i64() {
                        c.set_value(i.clamp(i32::MIN as i64, i32::MAX as i64) as i32);
                    }
                }
            },
            FieldControl::Real(c) => match value {
                DataValue::Null => c.set_value(0.0),
                other => {
                    if let Some(r) = other.as_f64() {
                        c.set_value(r);
                    }
                }
            },
            FieldControl::Bool(c) => {
                let b = match &value {
                    DataValue::Bool(b) => *b,
                    DataValue::Int(i) => *i != 0,
                    DataValue::Real(r) => *r != 0.0,
                    DataValue::Text(s) => s == "true" || s == "1",
                    DataValue::Null => false,
                    DataValue::Bytes(_) => false,
                };
                c.set_checked(b);
            }
            FieldControl::Date(c) => match &value {
                DataValue::Text(s) => match parse_iso_date(s) {
                    Some(d) => c.set_value(Some(d)),
                    None => c.set_value(None),
                },
                _ => c.set_value(None),
            },
            FieldControl::Combo { combo, keys } => match &value {
                DataValue::Null => combo.set_value(""),
                other => {
                    let key = other
                        .as_str()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| other.to_display_string());
                    let mut keys = keys.borrow_mut();
                    match keys.iter().position(|k| *k == key) {
                        Some(i) => combo.set_selection(i),
                        None => {
                            combo.append(&key);
                            keys.push(key);
                            combo.set_selection(keys.len() - 1);
                        }
                    }
                }
            },
            FieldControl::Blob { view, stored } => {
                let label = match &value {
                    DataValue::Null => String::new(),
                    DataValue::Bytes(b) => format!("[binario: {} byte]", b.len()),
                    other => other.to_display_string(),
                };
                view.set_label(&label);
                *stored = Some(value);
                let _ = column;
            }
        }
    }
}

// ── Campo (etichetta + controllo) ─────────────────────────────────────

/// Una riga del form: colonna dello schema, etichetta e controllo.
struct Field {
    column: DataColumn,
    label: StaticText,
    control: FieldControl,
}

impl Field {
    /// Widget etichetta (colonna 0 del sizer).
    fn label_widget(&self) -> WidgetRef {
        self.label.as_widget_ref()
    }

    /// Nome della colonna.
    fn name(&self) -> &str {
        &self.column.name
    }
}

// ── Layout salvato ────────────────────────────────────────────────────

/// Layout di un [`AutoForm`] serializzabile in JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormLayout {
    /// Versione dello schema di serializzazione.
    pub version: u32,
    /// Visibilità di ogni colonna, nell'ordine dello schema.
    pub columns: Vec<ColumnLayout>,
}

/// Stato di una colonna nel [`FormLayout`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnLayout {
    /// Nome della colonna.
    pub name: String,
    /// `true` se etichetta e campo sono visibili.
    pub visible: bool,
}

// ── AutoForm ──────────────────────────────────────────────────────────

/// Form modale generata automaticamente da uno [`TableSchema`].
///
/// Vedi il modulo per dettagli su controlli, layout e serializzazione.
pub struct AutoForm {
    schema: TableSchema,
    fields: Vec<Field>,
    grid: FlexGridSizer,
}

impl AutoForm {
    /// Genera il form per lo schema; le colonne con chiave esterna
    /// restano campi di testo (nessun lookup disponibile).
    pub fn new<W: Window>(parent: &W, schema: TableSchema) -> Self {
        Self::build(parent, schema, None)
    }

    /// Genera il form caricando le combo FK con `lookup`: per ogni
    /// vincolo la closure ritorna le coppie `(chiave, etichetta)` della
    /// tabella referenziata.
    pub fn with_fk_lookup<W: Window, F>(parent: &W, schema: TableSchema, lookup: F) -> Self
    where
        F: Fn(&ForeignKey) -> Vec<(String, String)> + 'static,
    {
        Self::build(parent, schema, Some(Rc::new(lookup)))
    }

    /// Costruzione condivisa dai due costruttori.
    fn build<W: Window>(parent: &W, schema: TableSchema, lookup: Option<FkLookup>) -> Self {
        let mut grid = FlexGridSizer::new(2, 8, 4);
        let mut fields = Vec::with_capacity(schema.columns.len());

        for col in &schema.columns {
            // Etichetta: nome colonna; le chiavi auto-incrementanti
            // vengono segnalate per chiarezza.
            let mut text = col.name.clone();
            if col.auto_increment {
                text.push_str(" (auto)");
            }
            let label = StaticText::new(parent, &text);

            // Prima FK della colonna (se presente).
            let fk = schema
                .foreign_keys_for(&col.name)
                .first()
                .map(|f| (*f).clone());

            let control = match fk {
                Some(fk) => match &lookup {
                    Some(lookup) => {
                        let combo = ComboBox::new(parent);
                        let mut keys = Vec::new();
                        for (key, display) in lookup(&fk) {
                            combo.append(&display);
                            keys.push(key);
                        }
                        FieldControl::Combo {
                            combo,
                            keys: Rc::new(RefCell::new(keys)),
                        }
                    }
                    // Nessun lookup: la chiave si compila a mano.
                    None => FieldControl::Text(TextCtrl::new(parent, "")),
                },
                None => match col.data_type {
                    DataFieldType::Text => FieldControl::Text(TextCtrl::new(parent, "")),
                    DataFieldType::Integer => {
                        // L'up-down Win32 gestisce posizioni a 16 bit
                        // (0..0x7FFF): usiamo un range pratico.
                        let spin = SpinCtrl::new(parent, 0, 0x7FFF, 0);
                        // Le chiavi auto-incrementanti non si modificano.
                        if col.auto_increment {
                            spin.as_widget_ref().borrow_mut().set_enabled(false);
                        }
                        FieldControl::Int(spin)
                    }
                    DataFieldType::Real => {
                        // Posizioni a 16 bit (max 32767 step): con
                        // step 0.1 e 2 decimali, ±1000 dà 20000 step.
                        FieldControl::Real(SpinCtrlDouble::new(
                            parent, 0.0, -1000.0, 1000.0, 0.1, 2,
                        ))
                    }
                    DataFieldType::Boolean => FieldControl::Bool(CheckBox::new(parent, "")),
                    DataFieldType::Date | DataFieldType::DateTime => FieldControl::Date(
                        DatePickerCtrl::new_with_format(parent, DateFormat::Short, col.nullable),
                    ),
                    DataFieldType::Blob => {
                        let view = StaticText::new(parent, "");
                        view.as_widget_ref().borrow_mut().set_enabled(false);
                        FieldControl::Blob { view, stored: None }
                    }
                },
            };

            grid.add(label.as_widget_ref());
            grid.add(control.widget());
            fields.push(Field {
                column: col.clone(),
                label,
                control,
            });
        }

        // La colonna dei campi cresce con il contenitore.
        grid.add_growable_col(1);

        AutoForm {
            schema,
            fields,
            grid,
        }
    }

    // ── Accesso allo schema ──────────────────────────────────────────

    /// Schema alla base del form.
    pub fn schema(&self) -> &TableSchema {
        &self.schema
    }

    /// Numero di campi generati.
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Tipo di controllo generato per la colonna (case-insensitive).
    pub fn control_kind(&self, column: &str) -> Option<ControlKind> {
        self.field(column).map(|f| f.control.kind())
    }

    /// Id nativo del controllo di una colonna, per
    /// `Frame::register_command_handler`; `None` se la colonna non
    /// esiste o non ha eventi (blob).
    pub fn control_id(&self, column: &str) -> Option<u16> {
        self.field(column).and_then(|f| f.control.id())
    }

    /// Riferimento type-erased del campo (etichetta + controllo),
    /// utile per posizionare l'eventuale event handler.
    pub fn widget(&self, column: &str) -> Option<WidgetRef> {
        self.field(column).map(|f| f.control.widget())
    }

    /// Colonna dello schema per nome (case-insensitive).
    pub fn column(&self, name: &str) -> Option<&DataColumn> {
        self.schema.column(name)
    }

    fn field(&self, column: &str) -> Option<&Field> {
        self.fields
            .iter()
            .find(|f| f.column.name.eq_ignore_ascii_case(column))
    }

    fn field_mut(&mut self, column: &str) -> Option<&mut Field> {
        self.fields
            .iter_mut()
            .find(|f| f.column.name.eq_ignore_ascii_case(column))
    }

    // ── Valori ───────────────────────────────────────────────────────

    /// Legge il valore di una colonna.
    pub fn get_value(&self, column: &str) -> Option<DataValue> {
        self.field(column).map(|f| f.control.get(&f.column))
    }

    /// Scrive il valore di una colonna.
    pub fn set_value(&mut self, column: &str, value: DataValue) {
        if let Some(f) = self.field_mut(column) {
            let col = f.column.clone();
            f.control.set(&col, value);
        }
    }

    /// Popola i campi con i valori di una riga, nell'ordine delle
    /// colonne dello schema (i valori mancanti diventano `Null`).
    pub fn set_row(&mut self, values: &[DataValue]) {
        for (i, field) in self.fields.iter_mut().enumerate() {
            let value = values.get(i).cloned().unwrap_or(DataValue::Null);
            let col = field.column.clone();
            field.control.set(&col, value);
        }
    }

    /// Legge i campi come riga di valori, nell'ordine delle colonne.
    pub fn get_row(&self) -> Vec<DataValue> {
        self.fields
            .iter()
            .map(|f| f.control.get(&f.column))
            .collect()
    }

    // ── Validazione ──────────────────────────────────────────────────

    /// Valida i campi visibili contro lo schema e restituisce la lista
    /// degli errori (vuota se tutto è valido).
    ///
    /// Regole:
    /// - colonna `NOT NULL` (non auto-incremento): il valore non può
    ///   essere `Null` né testo vuoto;
    /// - i campi nascosti (`set_column_visible(false)`) e le colonne
    ///   auto-incrementanti sono esclusi: non sono responsabilità
    ///   dell'utente.
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        for f in &self.fields {
            if f.column.auto_increment {
                continue;
            }
            if !f.control.widget().borrow().is_visible() {
                continue;
            }
            if f.column.nullable {
                continue;
            }
            let value = f.control.get(&f.column);
            let empty = match &value {
                DataValue::Null => true,
                DataValue::Text(t) => t.trim().is_empty(),
                _ => false,
            };
            if empty {
                errors.push(format!("{}: campo obbligatorio", f.column.name));
            }
        }
        errors
    }

    /// `true` se il form è valido (`validate()` senza errori).
    pub fn is_valid(&self) -> bool {
        self.validate().is_empty()
    }

    // ── Layout geometrico ────────────────────────────────────────────

    /// Posiziona il form nel rettangolo indicato (drive manuale, da
    /// chiamare nel gestore di ridimensionamento del contenitore).
    pub fn layout(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.grid
            .layout(x, y, width.max(0) as u32, height.max(0) as u32);
    }

    // ── Visibilità colonne e layout serializzabile ───────────────────

    /// Mostra/nasconde etichetta e campo di una colonna.
    pub fn set_column_visible(&self, column: &str, visible: bool) {
        if let Some(f) = self.field(column) {
            f.label_widget().borrow_mut().set_visible(visible);
            f.control.widget().borrow_mut().set_visible(visible);
        }
    }

    /// Stato di visibilità di una colonna (`None` se non esiste).
    pub fn is_column_visible(&self, column: &str) -> Option<bool> {
        self.field(column)
            .map(|f| f.label_widget().borrow().is_visible())
    }

    /// Serializza il layout corrente (visibilità colonne) in JSON.
    pub fn save_layout(&self) -> String {
        let layout = FormLayout {
            version: 1,
            columns: self
                .fields
                .iter()
                .map(|f| ColumnLayout {
                    name: f.column.name.clone(),
                    visible: f.label_widget().borrow().is_visible(),
                })
                .collect(),
        };
        serde_json::to_string(&layout).unwrap_or_else(|_| "{}".to_string())
    }

    /// Ripristina un layout salvato con [`AutoForm::save_layout`];
    /// le colonne assenti dal JSON vengono ignorate.
    pub fn load_layout(&mut self, json: &str) -> Result<(), String> {
        let layout: FormLayout =
            serde_json::from_str(json).map_err(|e| format!("layout non valido: {e}"))?;
        for col in &layout.columns {
            self.set_column_visible(&col.name, col.visible);
        }
        Ok(())
    }
}

// ── Helper data ───────────────────────────────────────────────────────

/// Formatta una [`Date`] come ISO `YYYY-MM-DD` (forma usata da
/// [`DataValue`] per i campi data).
fn format_iso_date(d: Date) -> String {
    format!("{:04}-{:02}-{:02}", d.year, d.month, d.day)
}

/// Analizza una data ISO `YYYY-MM-DD` (i separatori `-`, `/`, `T` o
/// spazio sono equivalenti: da `YYYY-MM-DD HH:MM:SS` usa solo la data).
fn parse_iso_date(s: &str) -> Option<Date> {
    let mut parts = s.trim().split(['-', '/', 'T', ' ']);
    let year: i32 = parts.next()?.trim().parse().ok()?;
    let month: u32 = parts.next()?.trim().parse().ok()?;
    let day: u32 = parts.next()?.trim().parse().ok()?;
    // Range validi: mese 1-12, giorno 1-31, anno 1-9999.
    if !(1..=12).contains(&month) || day == 0 || day > 31 || !(1..=9999).contains(&year) {
        return None;
    }
    Some(Date::new(year, month, day))
}

// ── Test ──────────────────────────────────────────────────────────────

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    use crate::data::column::DataFieldType;
    use crate::window::frame::Frame;

    /// Schema di test: id auto-increment, nome, prezzo, attivo,
    /// categoria_id con FK verso `categorie`.
    fn test_schema() -> TableSchema {
        let mut schema = TableSchema::new("prodotti");
        schema.columns = vec![
            DataColumn::new("id", DataFieldType::Integer)
                .with_primary_key()
                .with_auto_increment(),
            DataColumn::new("nome", DataFieldType::Text).with_not_null(),
            DataColumn::new("prezzo", DataFieldType::Real),
            DataColumn::new("attivo", DataFieldType::Boolean),
            DataColumn::new("categoria_id", DataFieldType::Integer),
        ];
        schema.primary_key = Some("id".to_string());
        schema
            .foreign_keys
            .push(ForeignKey::new("categoria_id", "categorie", "id").with_display_column("nome"));
        schema
    }

    /// Lookup FK di test: due categorie.
    fn test_lookup() -> impl Fn(&ForeignKey) -> Vec<(String, String)> {
        |_fk| {
            vec![
                ("1".to_string(), "Elettronica".to_string()),
                ("2".to_string(), "Casa".to_string()),
            ]
        }
    }

    #[test]
    fn costruzione_sceglie_il_controllo_per_tipo() {
        let frame = Frame::for_testing();
        let form = AutoForm::with_fk_lookup(&frame, test_schema(), test_lookup());

        assert_eq!(form.field_count(), 5);
        assert_eq!(form.control_kind("id"), Some(ControlKind::Integer));
        assert_eq!(form.control_kind("nome"), Some(ControlKind::Text));
        assert_eq!(form.control_kind("prezzo"), Some(ControlKind::Real));
        assert_eq!(form.control_kind("attivo"), Some(ControlKind::Boolean));
        // La colonna con FK diventa combo perché c'è il lookup.
        assert_eq!(form.control_kind("categoria_id"), Some(ControlKind::Combo));
        // Case-insensitive.
        assert_eq!(form.control_kind("NOME"), Some(ControlKind::Text));
        assert_eq!(form.control_kind("inesistente"), None);
    }

    #[test]
    fn round_trip_valori() {
        // Frame reale: i setter/getter Win32 (SetWindowTextW, SendMessage)
        // non funzionano con l'HWND nullo di `for_testing`.
        let frame = Frame::builder()
            .with_title("auto-form-test")
            .with_size(300, 240)
            .build();
        let mut form = AutoForm::with_fk_lookup(&frame, test_schema(), test_lookup());

        form.set_row(&[
            DataValue::Int(7),
            DataValue::Text("Mario".to_string()),
            DataValue::Real(19.9),
            DataValue::Bool(true),
            DataValue::Text("2".to_string()),
        ]);

        let row = form.get_row();
        assert_eq!(
            row,
            vec![
                DataValue::Int(7),
                DataValue::Text("Mario".to_string()),
                DataValue::Real(19.9),
                DataValue::Bool(true),
                DataValue::Text("2".to_string()),
            ]
        );

        // Colonna NOT NULL: il Null diventa stringa vuota.
        form.set_value("nome", DataValue::Null);
        assert_eq!(form.get_value("nome"), Some(DataValue::Text(String::new())));

        // Colonna nullable: il testo vuoto torna Null.
        let mut schema_nullable = TableSchema::new("note");
        schema_nullable.columns = vec![DataColumn::new("nota", DataFieldType::Text)];
        let form2 = AutoForm::new(&frame, schema_nullable);
        assert_eq!(form2.get_value("nota"), Some(DataValue::Null));
    }

    #[test]
    fn combo_fk_seleziona_la_chiave() {
        // Frame reale: la combo usa SendMessage (CB_SETCURSEL) che con
        // l'HWND nullo di `for_testing` non ha effetto.
        let frame = Frame::builder()
            .with_title("auto-form-test")
            .with_size(300, 240)
            .build();
        let mut form = AutoForm::with_fk_lookup(&frame, test_schema(), test_lookup());

        // Chiave presente nel lookup → selezione.
        form.set_value("categoria_id", DataValue::Text("2".to_string()));
        assert_eq!(
            form.get_value("categoria_id"),
            Some(DataValue::Text("2".to_string()))
        );

        // Chiave assente → viene aggiunta alla combo.
        form.set_value("categoria_id", DataValue::Text("99".to_string()));
        assert_eq!(
            form.get_value("categoria_id"),
            Some(DataValue::Text("99".to_string()))
        );
    }

    #[test]
    fn validate_segnala_i_campi_obbligatori_vuoti() {
        let frame = Frame::builder()
            .with_title("auto-form-validate")
            .with_size(300, 240)
            .build();
        let mut form = AutoForm::with_fk_lookup(&frame, test_schema(), test_lookup());

        // Form vuoto: "nome" è NOT NULL → errore; "id" (auto) e le
        // colonne nullable sono escluse.
        let errors = form.validate();
        assert_eq!(errors, vec!["nome: campo obbligatorio".to_string()]);
        assert!(!form.is_valid());

        // Compilando il campo obbligatorio il form diventa valido.
        form.set_value("nome", DataValue::Text("Mario".to_string()));
        assert!(form.validate().is_empty());
        assert!(form.is_valid());

        // Solo spazi: conta come vuoto.
        form.set_value("nome", DataValue::Text("   ".to_string()));
        assert_eq!(form.validate().len(), 1);

        // Campo nascosto: escluso dalla validazione.
        form.set_value("nome", DataValue::Null);
        form.set_column_visible("nome", false);
        assert!(form.validate().is_empty());
    }

    #[test]
    fn senza_lookup_restane_campo_testo() {
        let frame = Frame::for_testing();
        let form = AutoForm::new(&frame, test_schema());
        assert_eq!(form.control_kind("categoria_id"), Some(ControlKind::Text));
    }

    #[test]
    fn layout_serializza_e_ripristina_la_visibilita() {
        let frame = Frame::for_testing();
        let mut form = AutoForm::new(&frame, test_schema());

        assert_eq!(form.is_column_visible("attivo"), Some(true));
        form.set_column_visible("attivo", false);
        assert_eq!(form.is_column_visible("attivo"), Some(false));
        assert_eq!(form.is_column_visible("inesistente"), None);

        let json = form.save_layout();
        assert!(json.contains("attivo"));

        // Nascondo tutto, poi ripristino: "attivo" torna nascosto.
        form.set_column_visible("nome", false);
        form.load_layout(&json).expect("layout valido");
        assert_eq!(form.is_column_visible("attivo"), Some(false));
        assert_eq!(form.is_column_visible("nome"), Some(true));

        // JSON invalido → errore.
        assert!(form.load_layout("non-json").is_err());
    }

    #[test]
    fn layout_piazza_i_campi() {
        let frame = Frame::for_testing();
        let mut form = AutoForm::new(&frame, test_schema());

        form.layout(10, 20, 400, 300);

        // L'etichetta di "nome" (colonna 0) parte dall'origine richiesta,
        // mentre il suo campo (colonna 1) è spostato a destra.
        let label_x = form
            .field("nome")
            .map(|f| f.label_widget().borrow().rect().x)
            .expect("campo nome");
        let field_x = form.widget("nome").expect("widget nome").borrow().rect().x;
        assert!(label_x >= 10, "etichetta x = {label_x}");
        assert!(
            field_x > label_x,
            "campo x = {field_x}, etichetta x = {label_x}"
        );
    }

    #[test]
    fn id_dei_campi_sono_univoci() {
        let frame = Frame::for_testing();
        let form = AutoForm::new(&frame, test_schema());

        let mut ids: Vec<u16> = form.fields.iter().filter_map(|f| f.control.id()).collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "id duplicati nei campi del form");
    }
}
