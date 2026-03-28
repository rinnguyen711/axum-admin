use axum_admin::AdminError;
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
