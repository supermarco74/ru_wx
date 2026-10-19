//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Contratto dei provider: il trait `DataProvider` e il tipo `ProviderKind`.

use crate::data::error::DataError;
use crate::data::schema::TableSchema;
use crate::data::table::DataTable;
use crate::data::value::DataValue;

/// Tipo di backend sottostante un provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    /// SQLite (file o `:memory:`).
    Sqlite,
    /// MySQL 5.7 / 8.x.
    Mysql,
    /// MariaDB (rilevato dal banner di versione).
    MariaDb,
    /// Tabella in memoria, senza vero backend SQL.
    Memory,
}

impl ProviderKind {
    /// `true` se il backend è MariaDB.
    pub fn is_mariadb(&self) -> bool {
        matches!(self, ProviderKind::MariaDb)
    }

    /// Nome leggibile in italiano.
    pub fn label(&self) -> &'static str {
        match self {
            ProviderKind::Sqlite => "SQLite",
            ProviderKind::Mysql => "MySQL",
            ProviderKind::MariaDb => "MariaDB",
            ProviderKind::Memory => "Memoria",
        }
    }
}

/// Contratto unico per tutti i database (SQLite, MySQL, MariaDB, memoria).
///
/// API **sincrona**: nessun runtime async. Tutti i metodi che toccano il
/// backend restituiscono `Result<_, DataError>` (mai `unwrap`).
///
/// # Esempio
/// ```no_run
/// use ru_wx::data::{connect, DataValue};
///
/// let mut db = connect("sqlite::memory:").unwrap();
/// db.execute("CREATE TABLE clienti (id INTEGER PRIMARY KEY, nome TEXT)", &[])
///     .unwrap();
/// db.execute("INSERT INTO clienti (nome) VALUES (?)", &[DataValue::Text("Mario".into())])
///     .unwrap();
/// let tab = db.query("SELECT id, nome FROM clienti", &[]).unwrap();
/// let (headers, rows) = tab.to_string_matrix();
/// ```
pub trait DataProvider {
    /// Tipo di backend (rilevato o dichiarato).
    fn kind(&self) -> ProviderKind;

    /// Esegue una SELECT (o qualsiasi query che produce risultati) e
    /// restituisce una `DataTable`.
    fn query(&mut self, sql: &str, params: &[DataValue]) -> Result<DataTable, DataError>;

    /// Esegue un comando (INSERT/UPDATE/DELETE/DDL) e restituisce il
    /// numero di righe coinvolte.
    fn execute(&mut self, sql: &str, params: &[DataValue]) -> Result<u64, DataError>;

    /// Ultimo id generato da un INSERT con auto-incremento.
    fn last_insert_id(&mut self) -> Result<i64, DataError>;

    /// Introspezione: schema completo di una tabella (colonne, PK, FK,
    /// colonna-etichetta) per il futuro generatore di form.
    fn table_schema(&mut self, table: &str) -> Result<TableSchema, DataError>;

    /// Elenco delle tabelle visibili nel database corrente.
    fn tables(&mut self) -> Result<Vec<String>, DataError>;

    /// Quota un identificatore (nome tabella/colonna) per il backend.
    fn quote_ident(&self, ident: &str) -> String;

    /// Legge l'intera tabella con un filtro opzionale (`WHERE` già pronto,
    /// senza la parola `WHERE`).
    ///
    /// Scelta di progetto: sui DB reali è solo `SELECT * FROM <tabella>`
    /// (+ eventuale `WHERE <filter>`); la geometria della `DataTable`
    /// viene comunque ricostruita per intero. Su `MemoryProvider`, che non
    /// ha SQL, restituisce la tabella cachata e applica un filtro semplice
    /// `colonna operatore valore` (es. `id > 5`).
    fn query_table(&mut self, table: &str, filter: Option<&str>) -> Result<DataTable, DataError> {
        let qt = self.quote_ident(table);
        let mut sql = format!("SELECT * FROM {qt}");
        if let Some(f) = filter {
            if !f.trim().is_empty() {
                sql.push_str(" WHERE ");
                sql.push_str(f);
            }
        }
        self.query(&sql, &[])
    }
}
