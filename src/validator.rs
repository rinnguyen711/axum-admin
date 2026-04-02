use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;

use crate::adapter::DataAdapter;

/// Synchronous field validator. Implement this for custom validation logic.
pub trait Validator: Send + Sync {
    fn validate(&self, value: &str) -> Result<(), String>;
}

/// Asynchronous field validator. Used for validators that need DB access (e.g. Unique).
/// `record_id` is the current record's PK — pass `Some` on edit, `None` on create.
#[async_trait]
pub trait AsyncValidator: Send + Sync {
    async fn validate(&self, value: &str, record_id: Option<&Value>) -> Result<(), String>;
}

// --- Built-in sync validators ---

/// Fails if the value is an empty string. Auto-wired when `.required()` is called on a Field.
pub struct Required;

impl Validator for Required {
    fn validate(&self, value: &str) -> Result<(), String> {
        if value.trim().is_empty() {
            Err("This field is required.".to_string())
        } else {
            Ok(())
        }
    }
}

/// Fails if the value's character count is less than `n`.
pub struct MinLength(pub usize);

impl Validator for MinLength {
    fn validate(&self, value: &str) -> Result<(), String> {
        if value.len() < self.0 {
            Err(format!("Must be at least {} characters.", self.0))
        } else {
            Ok(())
        }
    }
}

/// Fails if the value's character count is greater than `n`.
pub struct MaxLength(pub usize);

impl Validator for MaxLength {
    fn validate(&self, value: &str) -> Result<(), String> {
        if value.len() > self.0 {
            Err(format!("Must be at most {} characters.", self.0))
        } else {
            Ok(())
        }
    }
}

/// Fails if the value cannot be parsed as f64 or is less than `n`.
pub struct MinValue(pub f64);

impl Validator for MinValue {
    fn validate(&self, value: &str) -> Result<(), String> {
        if value.trim().is_empty() {
            return Ok(());  // let Required handle empty
        }
        match value.parse::<f64>() {
            Ok(n) if n >= self.0 => Ok(()),
            Ok(_) => Err(format!("Must be at least {}.", self.0)),
            Err(_) => Err("Must be a valid number.".to_string()),
        }
    }
}

/// Fails if the value cannot be parsed as f64 or is greater than `n`.
pub struct MaxValue(pub f64);

impl Validator for MaxValue {
    fn validate(&self, value: &str) -> Result<(), String> {
        if value.trim().is_empty() {
            return Ok(());  // let Required handle empty
        }
        match value.parse::<f64>() {
            Ok(n) if n <= self.0 => Ok(()),
            Ok(_) => Err(format!("Must be at most {}.", self.0)),
            Err(_) => Err("Must be a valid number.".to_string()),
        }
    }
}

/// Fails if the value does not match `pattern`. The pattern is compiled once at construction.
pub struct RegexValidator {
    pattern: String,
    regex: Regex,
}

impl RegexValidator {
    /// Panics if `pattern` is not a valid regex. Use `.regex()` on `Field` which calls this.
    pub fn new(pattern: &str) -> Self {
        Self {
            regex: Regex::new(pattern).expect("invalid regex pattern"),
            pattern: pattern.to_string(),
        }
    }
}

impl Validator for RegexValidator {
    fn validate(&self, value: &str) -> Result<(), String> {
        if value.trim().is_empty() {
            return Ok(());  // let Required handle empty
        }
        if self.regex.is_match(value) {
            Ok(())
        } else {
            Err(format!("Must match pattern: {}", self.pattern))
        }
    }
}

/// Basic email format validator. Auto-wired for `Field::email()`.
pub struct EmailFormat;

impl Validator for EmailFormat {
    fn validate(&self, value: &str) -> Result<(), String> {
        if value.trim().is_empty() {
            return Ok(());  // let Required handle empty
        }
        // Check for exactly one '@', non-empty local part, non-empty domain with a '.'
        let parts: Vec<&str> = value.splitn(2, '@').collect();
        if parts.len() != 2 || parts[0].is_empty() || !parts[1].contains('.') || parts[1].starts_with('.') || parts[1].ends_with('.') {
            Err("Enter a valid email address.".to_string())
        } else {
            Ok(())
        }
    }
}

// --- Built-in async validators ---

/// Fails if another row in the DB already has `col = value`.
/// On edit, excludes the current record (matched by PK `id`).
pub struct Unique {
    adapter: Box<dyn DataAdapter>,
    col: String,
}

impl Unique {
    pub fn new(adapter: Box<dyn DataAdapter>, col: &str) -> Self {
        Self {
            adapter,
            col: col.to_string(),
        }
    }
}

#[async_trait]
impl AsyncValidator for Unique {
    async fn validate(&self, value: &str, record_id: Option<&Value>) -> Result<(), String> {
        if value.trim().is_empty() {
            return Ok(());  // let Required handle empty
        }
        use crate::adapter::ListParams;
        use std::collections::HashMap;

        let mut filters = HashMap::new();
        filters.insert(self.col.clone(), Value::String(value.to_string()));

        let params = ListParams {
            page: 1,
            per_page: 10,
            filters,
            ..Default::default()
        };

        let rows = self.adapter.list(params).await.unwrap_or_default();

        // Filter out the current record (on edit)
        let conflicts: Vec<_> = rows.iter().filter(|row| {
            if let Some(rid) = record_id {
                let row_id = row.get("id").map(|v| match v {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    other => other.to_string(),
                }).unwrap_or_default();
                let current_id = match rid {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    other => other.to_string(),
                };
                row_id != current_id
            } else {
                true
            }
        }).collect();

        if conflicts.is_empty() {
            Ok(())
        } else {
            Err(format!("This {} is already taken.", self.col))
        }
    }
}
