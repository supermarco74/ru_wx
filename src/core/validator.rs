//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Form validation (`wxValidator`) — lightweight trait.

/// Validates user input before commit (wx `wxValidator`).
pub trait Validator: Send {
    /// Return `true` when `value` is acceptable.
    fn validate(&self, value: &str) -> bool;

    /// Optional user-facing error (shown by callers).
    fn error_message(&self) -> &str {
        "Invalid input"
    }
}

/// Accept any non-empty trimmed string.
#[derive(Debug, Clone, Default)]
pub struct NonEmptyValidator {
    message: String,
}

impl NonEmptyValidator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_message(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Validator for NonEmptyValidator {
    fn validate(&self, value: &str) -> bool {
        !value.trim().is_empty()
    }

    fn error_message(&self) -> &str {
        if self.message.is_empty() {
            "Value cannot be empty"
        } else {
            &self.message
        }
    }
}

/// Numeric range validator for integer text fields.
#[derive(Debug, Clone)]
pub struct RangeValidator {
    min: i32,
    max: i32,
}

impl RangeValidator {
    pub fn new(min: i32, max: i32) -> Self {
        Self { min, max }
    }
}

impl Validator for RangeValidator {
    fn validate(&self, value: &str) -> bool {
        value
            .trim()
            .parse::<i32>()
            .map(|n| n >= self.min && n <= self.max)
            .unwrap_or(false)
    }

    fn error_message(&self) -> &str {
        "Value out of range"
    }
}

/// Accept any string (`wxGenericValidator`).
#[derive(Debug, Clone, Default)]
pub struct GenericValidator;

impl Validator for GenericValidator {
    fn validate(&self, _value: &str) -> bool {
        true
    }

    fn error_message(&self) -> &str {
        "Invalid value"
    }
}

/// Integer text validator (`wxIntegerValidator`).
#[derive(Debug, Clone, Default)]
pub struct IntegerValidator;

impl Validator for IntegerValidator {
    fn validate(&self, value: &str) -> bool {
        value.trim().parse::<i64>().is_ok()
    }

    fn error_message(&self) -> &str {
        "Enter a whole number"
    }
}

/// Floating-point text validator (`wxFloatingPointValidator`).
#[derive(Debug, Clone, Default)]
pub struct FloatingPointValidator;

impl Validator for FloatingPointValidator {
    fn validate(&self, value: &str) -> bool {
        value.trim().parse::<f64>().is_ok()
    }

    fn error_message(&self) -> &str {
        "Enter a number"
    }
}
