//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Valore cella: `DataValue`, il tipo comune a tutti i provider.

use std::fmt;

/// Valore di una singola cella, indipendente dal backend.
///
/// Serde è una dipendenza già obbligatoria del crate, quindi il derive
/// è sempre attivo e non richiede alcuna feature flag.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DataValue {
    /// Valore nullo (SQL NULL).
    Null,
    /// Intero a 64 bit con segno.
    Int(i64),
    /// Virgola mobile a 64 bit.
    Real(f64),
    /// Testo Unicode.
    Text(String),
    /// Booleano (rappresentato come intero 0/1 sui DB che lo richiedono).
    Bool(bool),
    /// BLOB / dati binari grezzi.
    Bytes(Vec<u8>),
}

impl DataValue {
    /// Restituisce il valore come `&str` se è testuale, altrimenti `None`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            DataValue::Text(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Converte il valore in `f64` quando possibile (int, real, bool, testo numerico).
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            DataValue::Int(v) => Some(*v as f64),
            DataValue::Real(v) => Some(*v),
            DataValue::Bool(v) => Some(if *v { 1.0 } else { 0.0 }),
            DataValue::Text(s) => s.trim().parse::<f64>().ok(),
            _ => None,
        }
    }

    /// Converte il valore in `i64` quando possibile.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            DataValue::Int(v) => Some(*v),
            DataValue::Real(v) => Some(*v as i64),
            DataValue::Bool(v) => Some(if *v { 1 } else { 0 }),
            DataValue::Text(s) => s.trim().parse::<i64>().ok(),
            _ => None,
        }
    }

    /// `true` se il valore è SQL NULL.
    pub fn is_null(&self) -> bool {
        matches!(self, DataValue::Null)
    }

    /// Rappresentazione testuale per la cella di una DataGrid.
    ///
    /// I NULL diventano stringa vuota, i BLOB mostrano la dimensione in byte.
    pub fn to_display_string(&self) -> String {
        match self {
            DataValue::Null => String::new(),
            DataValue::Int(v) => v.to_string(),
            DataValue::Real(v) => format_real(*v),
            DataValue::Text(s) => s.clone(),
            DataValue::Bool(v) => v.to_string(),
            DataValue::Bytes(b) => format!("<{} byte>", b.len()),
        }
    }

    /// Nome leggibile del tipo (utile per intestazioni e debug).
    pub fn type_name(&self) -> &'static str {
        match self {
            DataValue::Null => "null",
            DataValue::Int(_) => "intero",
            DataValue::Real(_) => "reale",
            DataValue::Text(_) => "testo",
            DataValue::Bool(_) => "booleano",
            DataValue::Bytes(_) => "binario",
        }
    }
}

/// Formatta un `f64` senza coda di zeri inutili (es. `3.0` → `3`, `2.50` → `2.5`).
fn format_real(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

impl From<i64> for DataValue {
    fn from(v: i64) -> Self {
        DataValue::Int(v)
    }
}

impl From<i32> for DataValue {
    fn from(v: i32) -> Self {
        DataValue::Int(v as i64)
    }
}

impl From<f64> for DataValue {
    fn from(v: f64) -> Self {
        DataValue::Real(v)
    }
}

impl From<&str> for DataValue {
    fn from(v: &str) -> Self {
        DataValue::Text(v.to_string())
    }
}

impl From<String> for DataValue {
    fn from(v: String) -> Self {
        DataValue::Text(v)
    }
}

impl From<bool> for DataValue {
    fn from(v: bool) -> Self {
        DataValue::Bool(v)
    }
}

impl fmt::Display for DataValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_display_string())
    }
}
