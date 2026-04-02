use std::fmt;

impl fmt::Debug for dyn crate::validator::Validator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Validator")
    }
}

impl fmt::Debug for dyn crate::validator::AsyncValidator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AsyncValidator")
    }
}

/// Escape hatch for fully custom HTML widget rendering.
pub trait Widget: Send + Sync {
    /// Render this widget as an HTML string for a form input.
    fn render_input(&self, name: &str, value: Option<&str>) -> String;
    /// Render this widget as a display value for list views.
    fn render_display(&self, value: Option<&str>) -> String;
}

impl fmt::Debug for dyn Widget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Widget")
    }
}

pub enum FieldType {
    Text,
    TextArea,
    Email,
    Password,
    Number,
    Float,
    Boolean,
    Date,
    DateTime,
    Select(Vec<(String, String)>),
    ForeignKey {
        adapter: Box<dyn crate::adapter::DataAdapter>,
        value_field: String,
        label_field: String,
        limit: Option<u64>,
        order_by: Option<String>,
    },
    ManyToMany {
        adapter: Box<dyn crate::adapter::ManyToManyAdapter>,
    },
    Json,
    Custom(Box<dyn Widget>),
}

impl std::fmt::Debug for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldType::Text => write!(f, "Text"),
            FieldType::TextArea => write!(f, "TextArea"),
            FieldType::Email => write!(f, "Email"),
            FieldType::Password => write!(f, "Password"),
            FieldType::Number => write!(f, "Number"),
            FieldType::Float => write!(f, "Float"),
            FieldType::Boolean => write!(f, "Boolean"),
            FieldType::Date => write!(f, "Date"),
            FieldType::DateTime => write!(f, "DateTime"),
            FieldType::Select(opts) => write!(f, "Select({} options)", opts.len()),
            FieldType::ForeignKey { value_field, label_field, limit, order_by, .. } => {
                write!(f, "ForeignKey {{ value_field: {value_field:?}, label_field: {label_field:?}, limit: {limit:?}, order_by: {order_by:?} }}")
            }
            FieldType::ManyToMany { .. } => write!(f, "ManyToMany(..)"),
            FieldType::Json => write!(f, "Json"),
            FieldType::Custom(_) => write!(f, "Custom(..)"),
        }
    }
}

#[derive(Debug)]
pub struct Field {
    pub name: String,
    pub label: String,
    pub field_type: FieldType,
    pub readonly: bool,
    pub hidden: bool,
    pub list_only: bool,
    pub form_only: bool,
    pub required: bool,
    pub help_text: Option<String>,
    pub validators: Vec<Box<dyn crate::validator::Validator>>,
    pub async_validators: Vec<Box<dyn crate::validator::AsyncValidator>>,
}

/// Capitalise first letter of a snake_case name for use as a default label.
pub(crate) fn default_label(name: &str) -> String {
    let mut chars = name.replace('_', " ").chars().collect::<Vec<_>>();
    if let Some(c) = chars.first_mut() {
        *c = c.to_uppercase().next().unwrap_or(*c);
    }
    chars.into_iter().collect()
}

impl Field {
    pub(crate) fn new(name: &str, field_type: FieldType) -> Self {
        Self {
            label: default_label(name),
            name: name.to_string(),
            field_type,
            readonly: false,
            hidden: false,
            list_only: false,
            form_only: false,
            required: false,
            help_text: None,
            validators: Vec::new(),
            async_validators: Vec::new(),
        }
    }

    pub fn text(name: &str) -> Self { Self::new(name, FieldType::Text) }
    pub fn textarea(name: &str) -> Self { Self::new(name, FieldType::TextArea) }
    pub fn email(name: &str) -> Self {
        let mut f = Self::new(name, FieldType::Email);
        f.validators.push(Box::new(crate::validator::EmailFormat));
        f
    }
    pub fn password(name: &str) -> Self { Self::new(name, FieldType::Password) }
    pub fn number(name: &str) -> Self { Self::new(name, FieldType::Number) }
    pub fn float(name: &str) -> Self { Self::new(name, FieldType::Float) }
    pub fn boolean(name: &str) -> Self { Self::new(name, FieldType::Boolean) }
    pub fn date(name: &str) -> Self { Self::new(name, FieldType::Date) }
    pub fn datetime(name: &str) -> Self { Self::new(name, FieldType::DateTime) }
    pub fn json(name: &str) -> Self { Self::new(name, FieldType::Json) }

    pub fn select(name: &str, options: Vec<(String, String)>) -> Self {
        Self::new(name, FieldType::Select(options))
    }

    pub fn foreign_key(
        name: &str,
        label: &str,
        adapter: Box<dyn crate::adapter::DataAdapter>,
        value_field: &str,
        label_field: &str,
    ) -> Self {
        let mut f = Self::new(name, FieldType::ForeignKey {
            adapter,
            value_field: value_field.to_string(),
            label_field: label_field.to_string(),
            limit: None,
            order_by: None,
        });
        f.label = label.to_string();
        f
    }

    pub fn custom(name: &str, widget: Box<dyn Widget>) -> Self {
        Self::new(name, FieldType::Custom(widget))
    }

    pub fn many_to_many(name: &str, adapter: Box<dyn crate::adapter::ManyToManyAdapter>) -> Self {
        Self::new(name, FieldType::ManyToMany { adapter })
    }

    // Modifiers — all consume self and return Self for chaining
    pub fn label(mut self, label: &str) -> Self { self.label = label.to_string(); self }
    pub fn readonly(mut self) -> Self { self.readonly = true; self }
    pub fn hidden(mut self) -> Self { self.hidden = true; self }
    pub fn list_only(mut self) -> Self { self.list_only = true; self }
    pub fn form_only(mut self) -> Self { self.form_only = true; self }
    pub fn required(mut self) -> Self {
        self.required = true;
        self.validators.push(Box::new(crate::validator::Required));
        self
    }
    pub fn help_text(mut self, text: &str) -> Self { self.help_text = Some(text.to_string()); self }

    /// Add a custom synchronous validator.
    pub fn validator(mut self, v: Box<dyn crate::validator::Validator>) -> Self {
        self.validators.push(v);
        self
    }

    /// Add a custom asynchronous validator (e.g. uniqueness checks).
    pub fn async_validator(mut self, v: Box<dyn crate::validator::AsyncValidator>) -> Self {
        self.async_validators.push(v);
        self
    }

    pub fn min_length(mut self, n: usize) -> Self {
        self.validators.push(Box::new(crate::validator::MinLength(n)));
        self
    }

    pub fn max_length(mut self, n: usize) -> Self {
        self.validators.push(Box::new(crate::validator::MaxLength(n)));
        self
    }

    pub fn min_value(mut self, n: f64) -> Self {
        self.validators.push(Box::new(crate::validator::MinValue(n)));
        self
    }

    pub fn max_value(mut self, n: f64) -> Self {
        self.validators.push(Box::new(crate::validator::MaxValue(n)));
        self
    }

    pub fn regex(mut self, pattern: &str) -> Self {
        self.validators.push(Box::new(crate::validator::RegexValidator::new(pattern)));
        self
    }

    pub fn unique(mut self, adapter: Box<dyn crate::adapter::DataAdapter>, col: &str) -> Self {
        self.async_validators.push(Box::new(crate::validator::Unique::new(adapter, col)));
        self
    }

    pub fn fk_limit(mut self, n: u64) -> Self {
        if let FieldType::ForeignKey { ref mut limit, .. } = self.field_type {
            *limit = Some(n);
        }
        self
    }

    pub fn fk_order_by(mut self, field: &str) -> Self {
        if let FieldType::ForeignKey { ref mut order_by, .. } = self.field_type {
            *order_by = Some(field.to_string());
        }
        self
    }
}
