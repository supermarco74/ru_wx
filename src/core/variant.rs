//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Dynamic value container (`wxVariant`).

/// Tagged union for property values (`wxVariant`).
#[derive(Debug, Clone, PartialEq)]
pub enum Variant {
    Null,
    Bool(bool),
    Int(i64),
    Double(f64),
    String(String),
}

impl Variant {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_double(&self) -> Option<f64> {
        match self {
            Self::Double(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(v) => Some(v),
            _ => None,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "bool",
            Self::Int(_) => "int",
            Self::Double(_) => "double",
            Self::String(_) => "string",
        }
    }
}

impl From<bool> for Variant {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i64> for Variant {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<f64> for Variant {
    fn from(value: f64) -> Self {
        Self::Double(value)
    }
}

impl From<&str> for Variant {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}
