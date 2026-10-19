//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Errori del layer data: un'unica enum `DataError` per tutti i provider.

use std::fmt;

/// Errori prodotti dal layer data (connessione, query, schema, backend).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataError {
    /// Fallimento della connessione al database.
    Connect(String),
    /// Fallimento di una interrogazione o di un comando SQL.
    Query(String),
    /// Fallimento nell'introspezione degli schemi (tabelle, colonne, chiavi).
    Schema(String),
    /// Operazione non supportata dal provider corrente.
    Unsupported(&'static str),
    /// Errore generico del backend sottostante.
    Backend(String),
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataError::Connect(msg) => write!(f, "errore di connessione: {msg}"),
            DataError::Query(msg) => write!(f, "errore di query: {msg}"),
            DataError::Schema(msg) => write!(f, "errore di schema: {msg}"),
            DataError::Unsupported(what) => write!(f, "operazione non supportata: {what}"),
            DataError::Backend(msg) => write!(f, "errore del backend: {msg}"),
        }
    }
}

impl std::error::Error for DataError {}

impl DataError {
    /// Costruttore comodo per errori di query.
    pub fn query(msg: impl Into<String>) -> Self {
        DataError::Query(msg.into())
    }

    /// Costruttore comodo per errori di connessione.
    pub fn connect(msg: impl Into<String>) -> Self {
        DataError::Connect(msg.into())
    }

    /// Costruttore comodo per errori di schema.
    pub fn schema(msg: impl Into<String>) -> Self {
        DataError::Schema(msg.into())
    }

    /// Costruttore comodo per errori generici del backend.
    pub fn backend(msg: impl Into<String>) -> Self {
        DataError::Backend(msg.into())
    }
}
