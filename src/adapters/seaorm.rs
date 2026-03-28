use sea_orm::Value as SeaValue;
use serde_json::{json, Value};

/// Convert a SeaORM Value to a serde_json Value for use in admin contexts.
#[allow(unreachable_patterns)]
pub fn sea_value_to_json(v: SeaValue) -> Value {
    match v {
        SeaValue::Bool(Some(b)) => json!(b),
        SeaValue::TinyInt(Some(n)) => json!(n),
        SeaValue::SmallInt(Some(n)) => json!(n),
        SeaValue::Int(Some(n)) => json!(n),
        SeaValue::BigInt(Some(n)) => json!(n),
        SeaValue::TinyUnsigned(Some(n)) => json!(n),
        SeaValue::SmallUnsigned(Some(n)) => json!(n),
        SeaValue::Unsigned(Some(n)) => json!(n),
        SeaValue::BigUnsigned(Some(n)) => json!(n),
        SeaValue::Float(Some(f)) => json!(f),
        SeaValue::Double(Some(f)) => json!(f),
        SeaValue::String(Some(s)) => json!(*s),
        SeaValue::Char(Some(c)) => json!(c.to_string()),
        SeaValue::Bytes(Some(b)) => json!(String::from_utf8_lossy(&b).to_string()),
        SeaValue::Json(Some(j)) => *j,
        SeaValue::ChronoDate(Some(d)) => json!(d.to_string()),
        SeaValue::ChronoTime(Some(t)) => json!(t.to_string()),
        SeaValue::ChronoDateTime(Some(dt)) => json!(dt.to_string()),
        SeaValue::ChronoDateTimeUtc(Some(dt)) => json!(dt.to_string()),
        SeaValue::ChronoDateTimeLocal(Some(dt)) => json!(dt.to_string()),
        SeaValue::ChronoDateTimeWithTimeZone(Some(dt)) => json!(dt.to_string()),
        SeaValue::Uuid(Some(u)) => json!(u.to_string()),
        // All None variants and unsupported types map to null
        _ => Value::Null,
    }
}

/// Convert a serde_json string value to a SeaORM string Value.
pub fn json_to_sea_string(v: &Value) -> SeaValue {
    match v {
        Value::String(s) => SeaValue::String(Some(Box::new(s.clone()))),
        Value::Null => SeaValue::String(None),
        other => SeaValue::String(Some(Box::new(other.to_string()))),
    }
}
