//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Provider in memoria: `MemoryProvider`, tabella senza backend SQL.

use crate::data::column::DataColumn;
use crate::data::error::DataError;
use crate::data::provider::{DataProvider, ProviderKind};
use crate::data::schema::TableSchema;
use crate::data::table::DataTable;
use crate::data::value::DataValue;

/// Provider senza database: custodisce `DataTable` già caricate.
///
/// Serve per testare la UI (DataGrid, futuri form) senza toccare file o
/// server, e come risultato cache di altre operazioni.
pub struct MemoryProvider {
    /// Tabelle memorizzate, in ordine di inserimento.
    tables: Vec<DataTable>,
    /// Schemi associati (colonne dedotte dalla tabella se non forniti).
    schemas: Vec<TableSchema>,
}

/// Crea un provider in memoria vuoto.
pub fn connect_memory() -> MemoryProvider {
    MemoryProvider {
        tables: Vec::new(),
        schemas: Vec::new(),
    }
}

impl MemoryProvider {
    /// Nuovo provider vuoto.
    pub fn new() -> Self {
        connect_memory()
    }

    /// Inserisce (o sostituisce) una tabella e ne deduce lo schema.
    pub fn insert_table(&mut self, table: DataTable) {
        let mut schema = TableSchema::new(table.table_name.clone());
        schema.columns = table.columns.clone();
        schema.primary_key = table
            .columns
            .iter()
            .find(|c| c.primary_key)
            .map(|c| c.name.clone());
        let _ = schema.guess_label_column();
        self.insert_table_with_schema(table, schema);
    }

    /// Inserisce una tabella con lo schema esplicito (FK incluse).
    pub fn insert_table_with_schema(&mut self, table: DataTable, schema: TableSchema) {
        let name = table.table_name.clone();
        if let Some(slot) = self.tables.iter_mut().position(|t| t.table_name == name) {
            self.tables[slot] = table;
            self.schemas[slot] = schema;
        } else {
            self.tables.push(table);
            self.schemas.push(schema);
        }
    }

    /// Ritorna una tabella per nome (case-insensitive), se presente.
    pub fn get_table(&self, name: &str) -> Option<&DataTable> {
        self.tables
            .iter()
            .find(|t| t.table_name.eq_ignore_ascii_case(name))
    }

    /// Applica un filtro semplice `colonna operatore valore` a una tabella.
    ///
    /// Operatori supportati: `=`, `!=`/`<>`, `>`, `>=`, `<`, `<=`.
    /// Operatori SQL base (`AND`, `OR`) e confronto con `NULL`/`'testo'`/numeri.
    fn apply_filter(table: &DataTable, filter: &str) -> Result<DataTable, DataError> {
        let f = filter.trim();
        if f.is_empty() {
            return Ok(table.clone());
        }
        // Split su AND (priorità uniforme, senza parentesi): sufficiente per
        // i filtri semplici della UI.
        let mut result = table.clone();
        for clause in f.split_inclusive(" AND ").filter(|s| !s.trim().is_empty()) {
            let clause = clause.trim().trim_end_matches("AND").trim();
            let row_filtered = DataTable {
                table_name: result.table_name.clone(),
                columns: result.columns.clone(),
                rows: result
                    .rows
                    .iter()
                    .filter(|row| Self::eval_clause(table, row, clause).unwrap_or_default())
                    .cloned()
                    .collect(),
            };
            result = row_filtered;
        }
        Ok(result)
    }

    /// Valuta una singola condizione `colonna op valore` su una riga.
    fn eval_clause(table: &DataTable, row: &[DataValue], clause: &str) -> Result<bool, DataError> {
        const OPS: [&str; 7] = [">=", "<=", "!=", "<>", "=", ">", "<"];
        for op in OPS {
            if let Some(pos) = clause.find(op) {
                let col = clause[..pos].trim();
                let val = clause[pos + op.len()..].trim();
                let idx = table
                    .column_index(col)
                    .ok_or_else(|| DataError::query(format!("colonna sconosciuta: {col}")))?;
                let cell = row.get(idx).cloned().unwrap_or(DataValue::Null);
                let expected = Self::parse_literal(val);
                return Self::compare(&cell, &expected, op);
            }
        }
        Err(DataError::query(format!(
            "filtro non riconosciuto: {clause}"
        )))
    }

    /// Converte un letterale SQL (`5`, `2.5`, `'testo'`, `NULL`) in `DataValue`.
    fn parse_literal(raw: &str) -> DataValue {
        if raw.eq_ignore_ascii_case("null") {
            return DataValue::Null;
        }
        if raw.len() >= 2 && raw.starts_with('\'') && raw.ends_with('\'') {
            return DataValue::Text(raw[1..raw.len() - 1].replace("''", "'"));
        }
        if let Ok(i) = raw.parse::<i64>() {
            return DataValue::Int(i);
        }
        if let Ok(r) = raw.parse::<f64>() {
            return DataValue::Real(r);
        }
        DataValue::Text(raw.to_string())
    }

    /// Confronto scalar tra cella e letterale (gli errori di confronto
    /// diventano `false`: meglio non vedere righe che non capiamo).
    fn compare(cell: &DataValue, expected: &DataValue, op: &str) -> Result<bool, DataError> {
        use std::cmp::Ordering;
        let ord = match (cell, expected) {
            (DataValue::Null, DataValue::Null) => Some(Ordering::Equal),
            (DataValue::Null, _) | (_, DataValue::Null) => None,
            (DataValue::Int(a), DataValue::Int(b)) => Some(a.cmp(b)),
            (DataValue::Text(a), DataValue::Text(b)) => Some(a.cmp(b)),
            (DataValue::Bool(a), DataValue::Bool(b)) => a.partial_cmp(b),
            _ => cell.as_f64().partial_cmp(&expected.as_f64()),
        };
        let Some(ord) = ord else {
            return Ok(matches!(op, "!=" | "<>"));
        };
        Ok(match op {
            "=" => ord == Ordering::Equal,
            "!=" | "<>" => ord != Ordering::Equal,
            ">" => ord == Ordering::Greater,
            ">=" => ord != Ordering::Less,
            "<" => ord == Ordering::Less,
            "<=" => ord != Ordering::Greater,
            _ => false,
        })
    }
}

impl Default for MemoryProvider {
    fn default() -> Self {
        connect_memory()
    }
}

impl DataProvider for MemoryProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Memory
    }

    fn query(&mut self, _sql: &str, _params: &[DataValue]) -> Result<DataTable, DataError> {
        Err(DataError::Unsupported(
            "MemoryProvider: usa query_table con il nome della tabella",
        ))
    }

    fn execute(&mut self, _sql: &str, _params: &[DataValue]) -> Result<u64, DataError> {
        Err(DataError::Unsupported(
            "MemoryProvider: usa insert_table per caricare i dati",
        ))
    }

    fn last_insert_id(&mut self) -> Result<i64, DataError> {
        Err(DataError::Unsupported(
            "MemoryProvider: nessun auto-incremento",
        ))
    }

    fn table_schema(&mut self, table: &str) -> Result<TableSchema, DataError> {
        match self
            .schemas
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(table))
        {
            Some(s) => Ok(s.clone()),
            None => Err(DataError::Schema(format!("tabella sconosciuta: {table}"))),
        }
    }

    fn tables(&mut self) -> Result<Vec<String>, DataError> {
        Ok(self.tables.iter().map(|t| t.table_name.clone()).collect())
    }

    fn quote_ident(&self, _ident: &str) -> String {
        // Nessun SQL vero: l'identificatore passa tale quale.
        String::new()
    }

    fn query_table(&mut self, table: &str, filter: Option<&str>) -> Result<DataTable, DataError> {
        let Some(t) = self.get_table(table) else {
            return Err(DataError::Schema(format!("tabella sconosciuta: {table}")));
        };
        match filter {
            Some(f) if !f.trim().is_empty() => Self::apply_filter(t, f),
            _ => Ok(t.clone()),
        }
    }
}

/// Colonne dedotte da nomi + righe (helper per costruire tabelle in memoria
/// senza conoscere i tipi a priori).
pub fn columns_from_rows(names: &[String], rows: &[Vec<DataValue>]) -> Vec<DataColumn> {
    let sample = rows.first().map(|r| r.as_slice());
    DataTable::infer_columns(names, sample)
}
