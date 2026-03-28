#[cfg(feature = "seaorm")]
mod tests {
    use axum_admin::adapters::seaorm::sea_value_to_json;
    use sea_orm::Value as SeaValue;
    use serde_json::{json, Value};

    #[test]
    fn sea_value_string_to_json() {
        let v = SeaValue::String(Some(Box::new("hello".to_string())));
        assert_eq!(sea_value_to_json(v), json!("hello"));
    }

    #[test]
    fn sea_value_null_string_to_json() {
        let v = SeaValue::String(None);
        assert_eq!(sea_value_to_json(v), Value::Null);
    }

    #[test]
    fn sea_value_int_to_json() {
        let v = SeaValue::Int(Some(42));
        assert_eq!(sea_value_to_json(v), json!(42));
    }

    #[test]
    fn sea_value_bool_to_json() {
        let v = SeaValue::Bool(Some(true));
        assert_eq!(sea_value_to_json(v), json!(true));
    }
}
