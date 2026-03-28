use std::fmt;

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

#[derive(Debug)]
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
    Relation {
        entity: String,
        display_field: String,
    },
    Json,
    Custom(Box<dyn Widget>),
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
    fn new(name: &str, field_type: FieldType) -> Self {
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
        }
    }

    pub fn text(name: &str) -> Self { Self::new(name, FieldType::Text) }
    pub fn textarea(name: &str) -> Self { Self::new(name, FieldType::TextArea) }
    pub fn email(name: &str) -> Self { Self::new(name, FieldType::Email) }
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

    pub fn relation(name: &str, entity: &str, display_field: &str) -> Self {
        Self::new(name, FieldType::Relation {
            entity: entity.to_string(),
            display_field: display_field.to_string(),
        })
    }

    pub fn custom(name: &str, widget: Box<dyn Widget>) -> Self {
        Self::new(name, FieldType::Custom(widget))
    }

    // Modifiers — all consume self and return Self for chaining
    pub fn label(mut self, label: &str) -> Self { self.label = label.to_string(); self }
    pub fn readonly(mut self) -> Self { self.readonly = true; self }
    pub fn hidden(mut self) -> Self { self.hidden = true; self }
    pub fn list_only(mut self) -> Self { self.list_only = true; self }
    pub fn form_only(mut self) -> Self { self.form_only = true; self }
    pub fn required(mut self) -> Self { self.required = true; self }
    pub fn help_text(mut self, text: &str) -> Self { self.help_text = Some(text.to_string()); self }
}
