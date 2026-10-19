//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Provider MySQL / MariaDB (feature `data-mysql`): `MySqlProvider` sul
//! crate `mysql` (API sincrona, nessun runtime async).

use mysql::prelude::Queryable;
use mysql::{Conn, Params, Value as MyValue};

use crate::data::column::{DataColumn, DataFieldType};
use crate::data::connection::extract_table_name;
use crate::data::error::DataError;
use crate::data::foreign_key::ForeignKey;
use crate::data::provider::{DataProvider, ProviderKind};
use crate::data::schema::TableSchema;
use crate::data::table::DataTable;
use crate::data::value::DataValue;

/// Provider MySQL / MariaDB basato sul crate `mysql`.
pub struct MySqlProvider {
    /// Connessione aperta (sincrona).
    conn: Conn,
    /// Variante rilevata con `SELECT VERSION()` (cache).
    variant: Option<ProviderKind>,
}

impl MySqlProvider {
    /// Apre una connessione con le opzioni fornite e rileva la variante
    /// (MySQL oppure MariaDB) leggendo `SELECT VERSION()`.
    pub fn connect(opts: mysql::Opts) -> Result<MySqlProvider, DataError> {
        let conn = Conn::new(opts).map_err(|e| DataError::connect(format!("MySQL: {e}")))?;
        let mut provider = MySqlProvider {
            conn,
            variant: None,
        };
        let kind = provider.detect_variant()?;
        provider.variant = Some(kind);
        Ok(provider)
    }

    /// Rileva (o rileva di nuovo) la variante del server.
    pub fn detect_variant(&mut self) -> Result<ProviderKind, DataError> {
        let sql = "SELECT VERSION()";
        let mut result = self
            .conn
            .exec_iter(sql, Params::Empty)
            .map_err(|e| DataError::query(format!("{sql}: {e}")))?;
        let row = match result.next() {
            Some(row) => row.map_err(|e| DataError::query(format!("{sql}: {e}")))?,
            None => {
                return Err(DataError::backend(
                    "SELECT VERSION() non ha restituito alcuna riga",
                ))
            }
        };
        let version = row.as_ref(0).and_then(value_as_string).unwrap_or_default();
        let kind = if version.to_lowercase().contains("mariadb") {
            ProviderKind::MariaDb
        } else {
            ProviderKind::Mysql
        };
        self.variant = Some(kind);
        Ok(kind)
    }

    /// Mappa un tipo dichiarato MySQL/MariaDB a un tipo logico.
    fn map_declared(decl: &str) -> DataFieldType {
        let t = decl.to_uppercase();
        if t.contains("BOOL") || t.starts_with("TINYINT(1)") {
            DataFieldType::Boolean
        } else if t.contains("DECIMAL")
            || t.contains("NUMERIC")
            || t.contains("FLOAT")
            || t.contains("DOUBLE")
            || t.contains("REAL")
        {
            DataFieldType::Real
        } else if t.contains("INT") || t.contains("YEAR") {
            DataFieldType::Integer
        } else if t.contains("CHAR")
            || t.contains("TEXT")
            || t.contains("ENUM")
            || t.contains("SET")
            || t.contains("JSON")
        {
            DataFieldType::Text
        } else if t.contains("DATETIME") || t.contains("TIMESTAMP") {
            DataFieldType::DateTime
        } else if t.contains("DATE") {
            DataFieldType::Date
        } else if t.contains("BINARY") || t.contains("BLOB") {
            DataFieldType::Blob
        } else {
            DataFieldType::Text
        }
    }
}

impl DataProvider for MySqlProvider {
    fn kind(&self) -> ProviderKind {
        self.variant.unwrap_or(ProviderKind::Mysql)
    }

    fn query(&mut self, sql: &str, params: &[DataValue]) -> Result<DataTable, DataError> {
        let result = self
            .conn
            .exec_iter(sql, to_params(params))
            .map_err(|e| DataError::query(format!("{sql}: {e}")))?;

        // Metadati delle colonne disponibili anche a zero righe.
        let names: Vec<String> = result
            .columns()
            .as_ref()
            .iter()
            .map(|c| c.name_str().to_string())
            .collect();

        let mut rows: Vec<Vec<DataValue>> = Vec::new();
        for row in result {
            let row = row.map_err(|e| DataError::query(format!("{sql}: {e}")))?;
            let mut cells = Vec::with_capacity(names.len());
            for i in 0..names.len() {
                cells.push(
                    row.as_ref(i)
                        .map(from_mysql_value)
                        .unwrap_or(DataValue::Null),
                );
            }
            rows.push(cells);
        }

        let sample = rows.first().map(Vec::as_slice);
        let columns = DataTable::infer_columns(&names, sample);
        let table_name = extract_table_name(sql);
        Ok(DataTable::from_rows(table_name, columns, rows))
    }

    fn execute(&mut self, sql: &str, params: &[DataValue]) -> Result<u64, DataError> {
        let result = self
            .conn
            .exec_iter(sql, to_params(params))
            .map_err(|e| DataError::query(format!("{sql}: {e}")))?;
        Ok(result.affected_rows())
    }

    fn last_insert_id(&mut self) -> Result<i64, DataError> {
        Ok(self.conn.last_insert_id() as i64)
    }

    fn table_schema(&mut self, table: &str) -> Result<TableSchema, DataError> {
        let mut schema = TableSchema::new(table);
        let table_param = || Params::Positional(vec![MyValue::Bytes(table.as_bytes().to_vec())]);

        // Colonne: information_schema.COLUMNS (DATABASE() = schema corrente).
        let sql = "SELECT COLUMN_NAME, COLUMN_TYPE, IS_NULLABLE, COLUMN_KEY, EXTRA, \
                   CHARACTER_MAXIMUM_LENGTH \
                   FROM information_schema.COLUMNS \
                   WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? \
                   ORDER BY ORDINAL_POSITION";
        let mut result = self
            .conn
            .exec_iter(sql, table_param())
            .map_err(|e| DataError::schema(format!("{sql}: {e}")))?;
        for row in result.by_ref() {
            let row = row.map_err(|e| DataError::schema(format!("{sql}: {e}")))?;
            let name = match row.as_ref(0).and_then(value_as_string) {
                Some(n) => n,
                None => continue,
            };
            let decl = row.as_ref(1).and_then(value_as_string).unwrap_or_default();
            let is_nullable = row
                .as_ref(2)
                .and_then(value_as_string)
                .map(|s| !s.eq_ignore_ascii_case("NO"))
                .unwrap_or(true);
            let key = row.as_ref(3).and_then(value_as_string).unwrap_or_default();
            let extra = row.as_ref(4).and_then(value_as_string).unwrap_or_default();
            let max_length = row.as_ref(5).and_then(value_as_u64);
            let is_pk = key.eq_ignore_ascii_case("PRI");

            let mut col = DataColumn::new(name.clone(), Self::map_declared(&decl));
            col.nullable = !is_nullable && !is_pk;
            col.primary_key = is_pk;
            col.auto_increment = extra.to_lowercase().contains("auto_increment");
            col.max_length = max_length.map(|n| n as usize);
            if is_pk && schema.primary_key.is_none() {
                schema.primary_key = Some(name.clone());
            }
            schema.columns.push(col);
        }
        // Il primo cursore deve essere rilasciato prima del secondo
        // prestito mutabile di `conn`.
        drop(result);

        // Chiavi esterne: KEY_COLUMN_USAGE (solo vincoli con tabella referenziata).
        let sql_fk = "SELECT COLUMN_NAME, REFERENCED_TABLE_NAME, REFERENCED_COLUMN_NAME \
                      FROM information_schema.KEY_COLUMN_USAGE \
                      WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? \
                        AND REFERENCED_TABLE_NAME IS NOT NULL \
                      ORDER BY ORDINAL_POSITION";
        let result_fk = self
            .conn
            .exec_iter(sql_fk, table_param())
            .map_err(|e| DataError::schema(format!("{sql_fk}: {e}")))?;
        for row in result_fk {
            let row = row.map_err(|e| DataError::schema(format!("{sql_fk}: {e}")))?;
            let column = match row.as_ref(0).and_then(value_as_string) {
                Some(c) => c,
                None => continue,
            };
            let ref_table = match row.as_ref(1).and_then(value_as_string) {
                Some(t) => t,
                None => continue,
            };
            if let Some(ref_column) = row.as_ref(2).and_then(value_as_string) {
                schema
                    .foreign_keys
                    .push(ForeignKey::new(column, ref_table, ref_column));
            }
        }

        schema.guess_label_column();
        Ok(schema)
    }

    fn tables(&mut self) -> Result<Vec<String>, DataError> {
        let sql = "SELECT TABLE_NAME FROM information_schema.TABLES \
                   WHERE TABLE_SCHEMA = DATABASE() AND TABLE_TYPE = 'BASE TABLE' \
                   ORDER BY TABLE_NAME";
        let result = self
            .conn
            .exec_iter(sql, Params::Empty)
            .map_err(|e| DataError::schema(format!("{sql}: {e}")))?;
        let mut out = Vec::new();
        for row in result {
            let row = row.map_err(|e| DataError::schema(format!("{sql}: {e}")))?;
            if let Some(name) = row.as_ref(0).and_then(value_as_string) {
                out.push(name);
            }
        }
        Ok(out)
    }

    fn quote_ident(&self, ident: &str) -> String {
        format!("`{}`", ident.replace('`', "``"))
    }
}

/// Converte i parametri generici nei valori posizionali di MySQL
/// (placeholder `?`, coerente con SQLite).
fn to_params(params: &[DataValue]) -> Params {
    let values: Vec<MyValue> = params.iter().map(to_mysql_value).collect();
    Params::Positional(values)
}

/// Converte un valore generico nel corrispondente valore MySQL.
fn to_mysql_value(v: &DataValue) -> MyValue {
    match v {
        DataValue::Null => MyValue::NULL,
        DataValue::Int(i) => MyValue::Int(*i),
        DataValue::Real(r) => MyValue::Double(*r),
        DataValue::Text(s) => MyValue::Bytes(s.as_bytes().to_vec()),
        DataValue::Bool(b) => MyValue::Int(if *b { 1 } else { 0 }),
        DataValue::Bytes(b) => MyValue::Bytes(b.clone()),
    }
}

/// Converte un valore MySQL nel tipo generico: i byte UTF-8 validi diventano
/// testo, gli altri restano binari; date e intervalli diventano stringhe.
fn from_mysql_value(v: &MyValue) -> DataValue {
    match v {
        MyValue::NULL => DataValue::Null,
        MyValue::Bytes(b) => match String::from_utf8(b.clone()) {
            Ok(s) => DataValue::Text(s),
            Err(_) => DataValue::Bytes(b.clone()),
        },
        MyValue::Int(i) => DataValue::Int(*i),
        MyValue::UInt(u) => {
            if *u <= i64::MAX as u64 {
                DataValue::Int(*u as i64)
            } else {
                DataValue::Text(u.to_string())
            }
        }
        MyValue::Float(f) => DataValue::Real(*f as f64),
        MyValue::Double(d) => DataValue::Real(*d),
        MyValue::Date(y, mo, d, h, mi, s, us) => {
            DataValue::Text(format_date(*y, *mo, *d, *h, *mi, *s, *us))
        }
        MyValue::Time(neg, days, h, mi, s, us) => {
            DataValue::Text(format_time(*neg, *days, *h, *mi, *s, *us))
        }
    }
}

/// Estrae una stringa leggibile da un valore MySQL (per gli schemi).
fn value_as_string(v: &MyValue) -> Option<String> {
    match v {
        MyValue::NULL => None,
        MyValue::Bytes(b) => Some(String::from_utf8_lossy(b).into_owned()),
        MyValue::Int(i) => Some(i.to_string()),
        MyValue::UInt(u) => Some(u.to_string()),
        MyValue::Float(f) => Some(f.to_string()),
        MyValue::Double(d) => Some(d.to_string()),
        MyValue::Date(y, mo, d, h, mi, s, us) => Some(format_date(*y, *mo, *d, *h, *mi, *s, *us)),
        MyValue::Time(neg, days, h, mi, s, us) => Some(format_time(*neg, *days, *h, *mi, *s, *us)),
    }
}

/// Estrae un intero non negativo da un valore MySQL, se possibile.
fn value_as_u64(v: &MyValue) -> Option<u64> {
    match v {
        MyValue::UInt(u) => Some(*u),
        MyValue::Int(i) if *i >= 0 => Some(*i as u64),
        MyValue::Bytes(b) => String::from_utf8_lossy(b).trim().parse().ok(),
        _ => None,
    }
}

/// Formatta una data/ora MySQL: solo data se mezzanotte esatta,
/// altrimenti data e ora (con microsecondi se presenti).
fn format_date(y: u16, mo: u8, d: u8, h: u8, mi: u8, s: u8, us: u32) -> String {
    if h == 0 && mi == 0 && s == 0 && us == 0 {
        format!("{y:04}-{mo:02}-{d:02}")
    } else if us == 0 {
        format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}")
    } else {
        format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}.{us:06}")
    }
}

/// Formatta un intervallo di tempo MySQL (`[-]ggg.hh:mm:ss[.ffffff]`).
fn format_time(neg: bool, days: u32, h: u8, m: u8, s: u8, us: u32) -> String {
    let hours = days * 24 + u32::from(h);
    let mut out = format!("{hours:02}:{m:02}:{s:02}");
    if us > 0 {
        out.push_str(&format!(".{us:06}"));
    }
    if neg {
        out.insert(0, '-');
    }
    out
}
