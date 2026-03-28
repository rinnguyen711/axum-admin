use axum_admin::AdminError;
use axum_admin::{Field, FieldType};
use std::collections::HashMap;

#[test]
fn admin_error_display() {
    let e = AdminError::NotFound;
    assert_eq!(e.to_string(), "not found");

    let mut fields = HashMap::new();
    fields.insert("email".to_string(), "is required".to_string());
    let e = AdminError::ValidationError(fields);
    assert!(e.to_string().contains("validation error"));

    let e = AdminError::DatabaseError("connection refused".to_string());
    assert!(e.to_string().contains("connection refused"));

    let e = AdminError::Unauthorized;
    assert_eq!(e.to_string(), "unauthorized");

    let e = AdminError::Custom("something went wrong".to_string());
    assert!(e.to_string().contains("something went wrong"));
}

#[test]
fn field_builder_text() {
    let f = Field::text("name");
    assert_eq!(f.name, "name");
    assert_eq!(f.label, "Name"); // auto-capitalised from field name
    assert!(!f.readonly);
    assert!(!f.hidden);
    assert!(!f.required);
    assert!(matches!(f.field_type, FieldType::Text));
}

#[test]
fn field_builder_chainable() {
    let f = Field::email("email")
        .label("Email Address")
        .required()
        .help_text("Must be unique");
    assert_eq!(f.label, "Email Address");
    assert!(f.required);
    assert_eq!(f.help_text, Some("Must be unique".to_string()));
    assert!(matches!(f.field_type, FieldType::Email));
}

#[test]
fn field_builder_select() {
    let f = Field::select(
        "status",
        vec![("active".to_string(), "Active".to_string()), ("banned".to_string(), "Banned".to_string())],
    );
    assert!(matches!(f.field_type, FieldType::Select(_)));
}

#[test]
fn field_modifiers() {
    let f = Field::number("age").readonly();
    assert!(f.readonly);

    let f = Field::text("secret").hidden();
    assert!(f.hidden);

    let f = Field::text("note").list_only();
    assert!(f.list_only);

    let f = Field::text("bio").form_only();
    assert!(f.form_only);
}
