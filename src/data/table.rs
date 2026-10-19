//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Tabelle dati: `DataTable`, il risultato di una query pronto per la DataGrid.

use crate::data::column::{DataColumn, DataFieldType};
use crate::data::value::DataValue;

/// Tabella di dati in memoria: colonne + righe piatte.
///
/// È il formato che la DataGrid consuma direttamente: `to_string_matrix`
/// produce intestazioni e celle già pronte per la visualizzazione.
#[derive(Debug, Clone, PartialEq)]
pub struct DataTable {
    /// Nome della tabella o dell'alias della query.
    pub table_name: String,
    /// Colonne (intestazioni) della tabella.
    pub columns: Vec<DataColumn>,
    /// Righe: ogni riga ha un valore per colonna, nell'ordine di `columns`.
    pub rows: Vec<Vec<DataValue>>,
}

impl DataTable {
    /// Crea una tabella vuota con solo il nome.
    pub fn new(table_name: impl Into<String>) -> Self {
        DataTable {
            table_name: table_name.into(),
            columns: Vec::new(),
            rows: Vec::new(),
        }
    }

    /// Crea una tabella da colonne e righe già allineate.
    ///
    /// Se una riga ha meno valori delle colonne viene riempita con `Null`;
    /// i valori in eccesso vengono scartati.
    pub fn from_rows(
        table_name: impl Into<String>,
        columns: Vec<DataColumn>,
        rows: Vec<Vec<DataValue>>,
    ) -> Self {
        let n = columns.len();
        let rows = rows
            .into_iter()
            .map(|mut r| {
                r.resize(n, DataValue::Null);
                r
            })
            .collect();
        DataTable {
            table_name: table_name.into(),
            columns,
            rows,
        }
    }

    /// Numero di righe.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Numero di colonne.
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    /// `true` se non contiene righe.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Cerca l'indice di una colonna per nome (case-insensitive).
    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.columns
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(name))
    }

    /// Ritorna la riga i-esima, se esiste.
    pub fn row(&self, i: usize) -> Option<&Vec<DataValue>> {
        self.rows.get(i)
    }

    /// Valore della cella (riga, colonna), se esiste.
    pub fn cell(&self, row: usize, column: usize) -> Option<&DataValue> {
        self.rows.get(row).and_then(|r| r.get(column))
    }

    /// Valore di una cella per nome di colonna, se esiste.
    pub fn cell_by_name(&self, row: usize, column: &str) -> Option<&DataValue> {
        self.column_index(column).and_then(|c| self.cell(row, c))
    }

    /// Intestazioni e righe come stringhe, pronte per la DataGrid.
    ///
    /// Restituisce `(intestazioni, righe)`: la prima è una riga di
    /// `column_count()` stringhe, la seconda contiene `row_count()` righe.
    pub fn to_string_matrix(&self) -> (Vec<String>, Vec<Vec<String>>) {
        let headers = self.columns.iter().map(|c| c.name.clone()).collect();
        let rows = self
            .rows
            .iter()
            .map(|r| r.iter().map(|v| v.to_display_string()).collect())
            .collect();
        (headers, rows)
    }

    /// Solo le righe come matrice di stringhe (senza intestazioni).
    pub fn rows_as_strings(&self) -> Vec<Vec<String>> {
        self.rows
            .iter()
            .map(|r| r.iter().map(|v| v.to_display_string()).collect())
            .collect()
    }

    /// Inferisce le colonne da una riga-campione (utile per i provider
    /// che conoscono solo i nomi delle colonne).
    pub fn infer_columns(names: &[String], sample: Option<&[DataValue]>) -> Vec<DataColumn> {
        names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let dtype = sample
                    .and_then(|row| row.get(i))
                    .map(DataFieldType::from_value)
                    .unwrap_or(DataFieldType::Text);
                DataColumn::new(name.clone(), dtype)
            })
            .collect()
    }
}
