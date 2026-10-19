//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Provider SQLite (feature `data-sqlite`): `SqliteProvider` su rusqlite.

use rusqlite::types::Value as SqlValue;

use crate::data::column::{DataColumn, DataFieldType};
use crate::data::connection::extract_table_name;
use crate::data::error::DataError;
use crate::data::foreign_key::ForeignKey;
use crate::data::provider::{DataProvider, ProviderKind};
use crate::data::schema::TableSchema;
use crate::data::table::DataTable;
use crate::data::value::DataValue;

/// Provider SQLite basato su rusqlite (bundled).
pub struct SqliteProvider {
    /// Connessione aperta (file o memoria).
    conn: rusqlite::Connection,
    /// Percorso del file (o `:memory:`), per debug.
    path: String,
}

impl SqliteProvider {
    /// Apre un database SQLite esistente (o lo crea se mancante).
    pub fn open(path: &str) -> Result<SqliteProvider, DataError> {
        let conn = rusqlite::Connection::open(path)
            .map_err(|e| DataError::connect(format!("SQLite {path}: {e}")))?;
        Ok(SqliteProvider {
            conn,
            path: path.to_string(),
        })
    }

    /// Apre un database in memoria (si perde alla chiusura).
    pub fn open_in_memory() -> Result<SqliteProvider, DataError> {
        let conn = rusqlite::Connection::open_in_memory()
            .map_err(|e| DataError::connect(format!("SQLite in memoria: {e}")))?;
        Ok(SqliteProvider {
            conn,
            path: ":memory:".to_string(),
        })
    }

    /// Percorso del database (o `:memory:`).
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Converte un valore generico nel tipo parametrico di SQLite.
    fn to_sql_value(v: &DataValue) -> SqlValue {
        match v {
            DataValue::Null => SqlValue::Null,
            DataValue::Int(i) => SqlValue::Integer(*i),
            DataValue::Real(r) => SqlValue::Real(*r),
            DataValue::Text(s) => SqlValue::Text(s.clone()),
            DataValue::Bool(b) => SqlValue::Integer(if *b { 1 } else { 0 }),
            DataValue::Bytes(b) => SqlValue::Blob(b.clone()),
        }
    }

    /// Converte un valore SQLite nel tipo generico.
    fn from_sql_value(v: SqlValue) -> DataValue {
        match v {
            SqlValue::Null => DataValue::Null,
            SqlValue::Integer(i) => DataValue::Int(i),
            SqlValue::Real(r) => DataValue::Real(r),
            SqlValue::Text(s) => DataValue::Text(s),
            SqlValue::Blob(b) => DataValue::Bytes(b),
        }
    }

    /// Mappa il tipo dichiarato SQLite a un tipo logico.
    fn map_declared(decl: &str) -> DataFieldType {
        let t = decl.to_uppercase();
        if t.contains("INT") {
            DataFieldType::Integer
        } else if t.contains("BOOL") {
            DataFieldType::Boolean
        } else if t.contains("CHAR") || t.contains("CLOB") || t.contains("TEXT") {
            DataFieldType::Text
        } else if t.contains("REAL") || t.contains("FLOA") || t.contains("DOUB") {
            DataFieldType::Real
        } else if t.contains("DATETIME") || t.contains("TIMESTAMP") {
            DataFieldType::DateTime
        } else if t.contains("DATE") {
            DataFieldType::Date
        } else if t.contains("BLOB") {
            DataFieldType::Blob
        } else {
            DataFieldType::Text
        }
    }
}

impl DataProvider for SqliteProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Sqlite
    }

    fn query(&mut self, sql: &str, params: &[DataValue]) -> Result<DataTable, DataError> {
        let values: Vec<SqlValue> = params.iter().map(Self::to_sql_value).collect();
        let mut stmt = self
            .conn
            .prepare(sql)
            .map_err(|e| DataError::query(format!("{sql}: {e}")))?;

        // Nomi delle colonne (i tipi dichiarati richiederebbero la feature
        // `column_decltype` di rusqlite, non attiva: si deducono dalla
        // prima riga, come per gli altri provider).
        let names: Vec<String> = stmt
            .column_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        let mut rows_iter = stmt
            .query(rusqlite::params_from_iter(values))
            .map_err(|e| DataError::query(format!("{sql}: {e}")))?;

        let mut rows: Vec<Vec<DataValue>> = Vec::new();
        while let Some(row) = rows_iter
            .next()
            .map_err(|e| DataError::query(format!("{sql}: {e}")))?
        {
            let mut cells = Vec::with_capacity(names.len());
            for i in 0..names.len() {
                let v: SqlValue = row
                    .get(i)
                    .map_err(|e| DataError::query(format!("{sql}: {e}")))?;
                cells.push(Self::from_sql_value(v));
            }
            rows.push(cells);
        }

        let sample = rows.first().map(Vec::as_slice);
        let columns = DataTable::infer_columns(&names, sample);
        let table_name = extract_table_name(sql);
        Ok(DataTable::from_rows(table_name, columns, rows))
    }

    fn execute(&mut self, sql: &str, params: &[DataValue]) -> Result<u64, DataError> {
        let values: Vec<SqlValue> = params.iter().map(Self::to_sql_value).collect();
        let n = self
            .conn
            .execute(sql, rusqlite::params_from_iter(values))
            .map_err(|e| DataError::query(format!("{sql}: {e}")))?;
        Ok(n as u64)
    }

    fn last_insert_id(&mut self) -> Result<i64, DataError> {
        Ok(self.conn.last_insert_rowid())
    }

    fn table_schema(&mut self, table: &str) -> Result<TableSchema, DataError> {
        let qt = self.quote_ident(table);

        // Colonne: PRAGMA table_info(t) → cid, name, type, notnull, dflt_value, pk
        let mut schema = TableSchema::new(table);
        let sql = format!("PRAGMA table_info({qt})");
        let mut stmt = self
            .conn
            .prepare(&sql)
            .map_err(|e| DataError::schema(format!("{sql}: {e}")))?;
        let mut info_rows = stmt
            .query([])
            .map_err(|e| DataError::schema(format!("{sql}: {e}")))?;

        // pk > 0 per le colonne della chiave primaria (1 = prima, 2 = seconda…)
        let mut pk_cols: Vec<(i64, String)> = Vec::new();
        while let Some(row) = info_rows
            .next()
            .map_err(|e| DataError::schema(format!("{sql}: {e}")))?
        {
            let name: String = row
                .get(1)
                .map_err(|e| DataError::schema(format!("{sql}: {e}")))?;
            let decl: String = row
                .get(2)
                .map_err(|e| DataError::schema(format!("{sql}: {e}")))?;
            let notnull: i64 = row
                .get(3)
                .map_err(|e| DataError::schema(format!("{sql}: {e}")))?;
            let pk: i64 = row
                .get(5)
                .map_err(|e| DataError::schema(format!("{sql}: {e}")))?;
            let mut col = DataColumn::new(name.clone(), Self::map_declared(&decl));
            col.nullable = notnull == 0 && pk == 0;
            col.primary_key = pk > 0;
            col.auto_increment = pk == 1 && decl.to_uppercase().contains("INT");
            if pk > 0 {
                pk_cols.push((pk, name));
            }
            schema.columns.push(col);
        }
        pk_cols.sort();
        schema.primary_key = pk_cols.first().map(|(_, n)| n.clone());

        // Chiavi esterne: PRAGMA foreign_key_list(t) → id, seq, table, from, to, …
        let sql_fk = format!("PRAGMA foreign_key_list({qt})");
        let mut stmt_fk = self
            .conn
            .prepare(&sql_fk)
            .map_err(|e| DataError::schema(format!("{sql_fk}: {e}")))?;
        let mut fk_rows = stmt_fk
            .query([])
            .map_err(|e| DataError::schema(format!("{sql_fk}: {e}")))?;
        while let Some(row) = fk_rows
            .next()
            .map_err(|e| DataError::schema(format!("{sql_fk}: {e}")))?
        {
            let ref_table: String = row
                .get(2)
                .map_err(|e| DataError::schema(format!("{sql_fk}: {e}")))?;
            let column: String = row
                .get(3)
                .map_err(|e| DataError::schema(format!("{sql_fk}: {e}")))?;
            let ref_column: Option<String> = row
                .get(4)
                .map_err(|e| DataError::schema(format!("{sql_fk}: {e}")))?;
            if let Some(ref_column) = ref_column {
                schema
                    .foreign_keys
                    .push(ForeignKey::new(column, ref_table, ref_column));
            }
        }

        schema.guess_label_column();
        Ok(schema)
    }

    fn tables(&mut self) -> Result<Vec<String>, DataError> {
        let sql = "SELECT name FROM sqlite_master \
                   WHERE type = 'table' AND name NOT LIKE 'sqlite_%' \
                   ORDER BY name";
        let mut stmt = self
            .conn
            .prepare(sql)
            .map_err(|e| DataError::schema(format!("{sql}: {e}")))?;
        let names = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| DataError::schema(format!("{sql}: {e}")))?;
        let mut out = Vec::new();
        for n in names {
            out.push(n.map_err(|e| DataError::schema(format!("{sql}: {e}")))?);
        }
        Ok(out)
    }

    fn quote_ident(&self, ident: &str) -> String {
        format!("\"{}\"", ident.replace('"', "\"\""))
    }
}
