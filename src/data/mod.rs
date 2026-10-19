//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Layer data: un solo punto d'ingresso (`connect`) per SQLite, MySQL,
//! MariaDB e memoria, tutti con la stessa API (`DataProvider`).
//!
//! - `value` / `column` / `table`: valori, colonne e `DataTable` pronta
//!   per la DataGrid (`to_string_matrix`);
//! - `foreign_key` / `schema`: chiavi esterne e `TableSchema` per il
//!   futuro generatore di form;
//! - `provider`: trait `DataProvider` e `ProviderKind`;
//! - `memory`, `sqlite` (feature `data-sqlite`), `mysql` (feature
//!   `data-mysql`): i provider concreti;
//! - `connection`: apertura da URL e parsing;
//! - `error`: un'unica enum `DataError`.

pub mod column;
pub mod connection;
pub mod error;
pub mod foreign_key;
pub mod memory;
pub mod provider;
pub mod schema;
pub mod sql_gen;
pub mod table;
pub mod value;

#[cfg(feature = "data-sqlite")]
pub mod sqlite;

#[cfg(feature = "data-mysql")]
pub mod mysql;

pub use column::{DataColumn, DataFieldType};
pub use connection::{connect, extract_table_name};
pub use error::DataError;
pub use foreign_key::ForeignKey;
pub use memory::{columns_from_rows, connect_memory, MemoryProvider};
pub use provider::{DataProvider, ProviderKind};
pub use schema::TableSchema;
pub use sql_gen::{build_insert, build_update};
pub use table::DataTable;
pub use value::DataValue;

#[cfg(feature = "data-sqlite")]
pub use sqlite::SqliteProvider;

#[cfg(feature = "data-mysql")]
pub use mysql::MySqlProvider;
