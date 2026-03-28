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

use crate::{
    adapter::{DataAdapter, ListParams},
    error::AdminError,
};
use async_trait::async_trait;
use sea_orm::{
    sea_query::{Condition, Expr},
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, IdenStatic, PaginatorTrait, QueryFilter,
    QueryResult, Statement, TryGetable,
};
use std::{collections::HashMap, marker::PhantomData};

/// Convert a serde_json Value to a SeaORM bind Value.
fn json_to_sea_value(v: &Value) -> SeaValue {
    match v {
        Value::String(s) => SeaValue::String(Some(Box::new(s.clone()))),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                SeaValue::BigInt(Some(i))
            } else if let Some(f) = n.as_f64() {
                SeaValue::Double(Some(f))
            } else {
                SeaValue::String(Some(Box::new(n.to_string())))
            }
        }
        Value::Bool(b) => SeaValue::Bool(Some(*b)),
        Value::Null => SeaValue::String(None),
        other => SeaValue::String(Some(Box::new(other.to_string()))),
    }
}

/// Convert a QueryResult row into a HashMap by trying typed extractors in order.
fn query_result_to_map(row: &QueryResult) -> HashMap<String, Value> {
    let cols = row.column_names();
    let mut map = HashMap::new();
    for col in &cols {
        // Try i64 first (covers INTEGER, BIGINT), then f64 (REAL/FLOAT), then String (TEXT)
        let v = if let Ok(i) = i64::try_get_by(row, col.as_str()) {
            json!(i)
        } else if let Ok(f) = f64::try_get_by(row, col.as_str()) {
            json!(f)
        } else if let Ok(s) = String::try_get_by(row, col.as_str()) {
            json!(s)
        } else {
            Value::Null
        };
        map.insert(col.clone(), v);
    }
    map
}

pub struct SeaOrmAdapter<E: EntityTrait> {
    db: DatabaseConnection,
    search_columns: Vec<String>,
    _marker: PhantomData<E>,
}

impl<E: EntityTrait> SeaOrmAdapter<E> {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            search_columns: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub fn search_columns(mut self, cols: Vec<String>) -> Self {
        self.search_columns = cols;
        self
    }
}

#[async_trait]
impl<E> DataAdapter for SeaOrmAdapter<E>
where
    E: EntityTrait + Default + Send + Sync,
    E::Model: Send + Sync,
{
    async fn list(&self, params: ListParams) -> Result<Vec<HashMap<String, Value>>, AdminError> {
        let table = sea_orm::EntityName::table_name(&E::default()).to_string();

        let mut bind_vals: Vec<SeaValue> = Vec::new();

        // Prefer search_columns from params (populated from entity.search_fields),
        // fall back to the adapter-level search_columns set via .search_columns().
        let search_cols = if !params.search_columns.is_empty() {
            &params.search_columns
        } else {
            &self.search_columns
        };

        let where_clause = match &params.search {
            Some(search) if !search.is_empty() && !search_cols.is_empty() => {
                let clauses: Vec<String> =
                    search_cols.iter().map(|c| format!("{} LIKE ?", c)).collect();
                for _ in search_cols {
                    bind_vals
                        .push(SeaValue::String(Some(Box::new(format!("%{}%", search)))));
                }
                format!(" WHERE {}", clauses.join(" OR "))
            }
            _ => String::new(),
        };

        let order_clause = match &params.order_by {
            Some((col, crate::adapter::SortOrder::Desc)) => {
                format!(" ORDER BY {} DESC", col)
            }
            Some((col, crate::adapter::SortOrder::Asc)) => {
                format!(" ORDER BY {} ASC", col)
            }
            None => String::new(),
        };

        let offset = params.page.saturating_sub(1) * params.per_page;
        let sql = format!(
            "SELECT * FROM {}{}{} LIMIT ? OFFSET ?",
            table, where_clause, order_clause
        );
        bind_vals.push(SeaValue::BigInt(Some(params.per_page as i64)));
        bind_vals.push(SeaValue::BigInt(Some(offset as i64)));

        let stmt = Statement::from_sql_and_values(self.db.get_database_backend(), &sql, bind_vals);
        let rows = self
            .db
            .query_all(stmt)
            .await
            .map_err(|e| AdminError::DatabaseError(e.to_string()))?;

        Ok(rows.iter().map(query_result_to_map).collect())
    }

    async fn get(&self, id: &Value) -> Result<HashMap<String, Value>, AdminError> {
        let table = sea_orm::EntityName::table_name(&E::default()).to_string();
        let id_val = json_to_sea_value(id);
        let sql = format!("SELECT * FROM {} WHERE id = ? LIMIT 1", table);
        let stmt = Statement::from_sql_and_values(self.db.get_database_backend(), &sql, [id_val]);
        let result = self
            .db
            .query_one(stmt)
            .await
            .map_err(|e| AdminError::DatabaseError(e.to_string()))?
            .ok_or(AdminError::NotFound)?;
        Ok(query_result_to_map(&result))
    }

    async fn create(&self, data: HashMap<String, Value>) -> Result<Value, AdminError> {
        let table = sea_orm::EntityName::table_name(&E::default()).to_string();
        let mut cols: Vec<String> = data.keys().cloned().collect();
        cols.sort();
        let placeholders = cols.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table,
            cols.join(", "),
            placeholders,
        );
        let vals: Vec<SeaValue> = cols
            .iter()
            .map(|k| json_to_sea_value(data.get(k).unwrap()))
            .collect();
        let stmt = Statement::from_sql_and_values(self.db.get_database_backend(), &sql, vals);
        let res = self
            .db
            .execute(stmt)
            .await
            .map_err(|e| AdminError::DatabaseError(e.to_string()))?;
        Ok(Value::Number(res.last_insert_id().into()))
    }

    async fn update(&self, id: &Value, data: HashMap<String, Value>) -> Result<(), AdminError> {
        let table = sea_orm::EntityName::table_name(&E::default()).to_string();
        let mut cols: Vec<String> = data.keys().cloned().collect();
        cols.sort();
        let set_clause = cols
            .iter()
            .map(|c| format!("{} = ?", c))
            .collect::<Vec<_>>()
            .join(", ");
        let mut vals: Vec<SeaValue> = cols
            .iter()
            .map(|k| json_to_sea_value(data.get(k).unwrap()))
            .collect();
        vals.push(json_to_sea_value(id));
        let sql = format!("UPDATE {} SET {} WHERE id = ?", table, set_clause);
        let stmt = Statement::from_sql_and_values(self.db.get_database_backend(), &sql, vals);
        self.db
            .execute(stmt)
            .await
            .map_err(|e| AdminError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn delete(&self, id: &Value) -> Result<(), AdminError> {
        let table = sea_orm::EntityName::table_name(&E::default()).to_string();
        let id_val = json_to_sea_value(id);
        let sql = format!("DELETE FROM {} WHERE id = ?", table);
        let stmt = Statement::from_sql_and_values(self.db.get_database_backend(), &sql, [id_val]);
        self.db
            .execute(stmt)
            .await
            .map_err(|e| AdminError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn count(&self, params: &ListParams) -> Result<u64, AdminError> {
        let mut query = E::find();

        let search_cols = if !params.search_columns.is_empty() {
            &params.search_columns
        } else {
            &self.search_columns
        };

        if let Some(ref search) = params.search {
            if !search_cols.is_empty() && !search.is_empty() {
                let mut cond = Condition::any();
                for col_name in search_cols {
                    cond = cond.add(
                        Expr::col(sea_orm::sea_query::Alias::new(col_name.as_str()))
                            .like(format!("%{}%", search)),
                    );
                }
                query = query.filter(cond);
            }
        }

        query
            .count(&self.db)
            .await
            .map_err(|e| AdminError::DatabaseError(e.to_string()))
    }
}

use crate::field::{Field, FieldType};

pub fn seaorm_fields_for<E>() -> Vec<Field>
where
    E: EntityTrait,
    E::Column: ColumnTrait,
{
    use sea_orm::sea_query::ColumnType;
    use sea_orm::Iterable;
    E::Column::iter()
        .map(|col| {
            let name = col.as_str();
            let col_def = col.def();
            let field_type = match col_def.get_column_type() {
                ColumnType::Char(_) | ColumnType::String(_) | ColumnType::Text => FieldType::Text,
                ColumnType::TinyInteger
                | ColumnType::SmallInteger
                | ColumnType::Integer
                | ColumnType::BigInteger
                | ColumnType::TinyUnsigned
                | ColumnType::SmallUnsigned
                | ColumnType::Unsigned
                | ColumnType::BigUnsigned => FieldType::Number,
                ColumnType::Float | ColumnType::Double | ColumnType::Decimal(_) => FieldType::Float,
                ColumnType::Boolean => FieldType::Boolean,
                ColumnType::Date => FieldType::Date,
                ColumnType::DateTime
                | ColumnType::Timestamp
                | ColumnType::TimestampWithTimeZone => FieldType::DateTime,
                ColumnType::Json | ColumnType::JsonBinary => FieldType::Json,
                ColumnType::Enum { variants, .. } => {
                    FieldType::Select(
                        variants.iter()
                            .map(|v| {
                                let s = v.to_string();
                                let label = crate::field::default_label(&s);
                                (s, label)
                            })
                            .collect()
                    )
                }
                _ => FieldType::Text,
            };
            let mut f = Field::new(name, field_type);
            if name == "id" {
                f = f.readonly();
            }
            f
        })
        .collect()
}
