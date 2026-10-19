//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Colonne: `DataColumn` e `DataFieldType`, metadati di ogni colonna.

use crate::data::value::DataValue;

/// Tipo logico di una colonna, indipendente dal backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataFieldType {
    /// Intero (INTEGER, INT, BIGINT, …).
    Integer,
    /// Virgola mobile (REAL, FLOAT, DOUBLE, DECIMAL, …).
    Real,
    /// Testo (TEXT, VARCHAR, CHAR, …).
    Text,
    /// Booleano (BOOL, BIT o intero 0/1).
    Boolean,
    /// Data senza ora (DATE).
    Date,
    /// Data con ora (DATETIME, TIMESTAMP).
    DateTime,
    /// BLOB / binario (BLOB, BINARY, VARBINARY).
    Blob,
}

impl DataFieldType {
    /// Nome leggibile in italiano del tipo.
    pub fn label(&self) -> &'static str {
        match self {
            DataFieldType::Integer => "intero",
            DataFieldType::Real => "reale",
            DataFieldType::Text => "testo",
            DataFieldType::Boolean => "booleano",
            DataFieldType::Date => "data",
            DataFieldType::DateTime => "data e ora",
            DataFieldType::Blob => "binario",
        }
    }

    /// Inferisce il tipo logico da un valore campione.
    pub fn from_value(value: &DataValue) -> DataFieldType {
        match value {
            DataValue::Null => DataFieldType::Text,
            DataValue::Int(_) => DataFieldType::Integer,
            DataValue::Real(_) => DataFieldType::Real,
            DataValue::Text(_) => DataFieldType::Text,
            DataValue::Bool(_) => DataFieldType::Boolean,
            DataValue::Bytes(_) => DataFieldType::Blob,
        }
    }
}

/// Metadati di una colonna di una `DataTable` / `TableSchema`.
#[derive(Debug, Clone, PartialEq)]
pub struct DataColumn {
    /// Nome della colonna così come appare nel database.
    pub name: String,
    /// Tipo logico della colonna.
    pub data_type: DataFieldType,
    /// `true` se la colonna ammette SQL NULL.
    pub nullable: bool,
    /// `true` se la colonna appartiene alla chiave primaria.
    pub primary_key: bool,
    /// `true` se la colonna è auto-incrementante (ROWID / AUTO_INCREMENT).
    pub auto_increment: bool,
    /// Lunghezza massima per le colonne testuali (VARCHAR(n)).
    pub max_length: Option<usize>,
}

impl DataColumn {
    /// Crea una colonna con i campi essenziali; gli altri campi usano default sensati.
    pub fn new(name: impl Into<String>, data_type: DataFieldType) -> Self {
        DataColumn {
            name: name.into(),
            data_type,
            nullable: true,
            primary_key: false,
            auto_increment: false,
            max_length: None,
        }
    }

    /// Marca la colonna come chiave primaria (e la rende non nullabile).
    pub fn with_primary_key(mut self) -> Self {
        self.primary_key = true;
        self.nullable = false;
        self
    }

    /// Marca la colonna come auto-incrementante.
    pub fn with_auto_increment(mut self) -> Self {
        self.auto_increment = true;
        self
    }

    /// Rende la colonna non nullabile.
    pub fn with_not_null(mut self) -> Self {
        self.nullable = false;
        self
    }

    /// Imposta la lunghezza massima (per colonne testuali).
    pub fn with_max_length(mut self, len: usize) -> Self {
        self.max_length = Some(len);
        self
    }
}
