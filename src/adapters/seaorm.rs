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
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, IdenStatic, PaginatorTrait, QueryFilter,
    QueryResult, Statement, TryGetable,
};

/// Replace `?` placeholders with `$1`, `$2`, … for PostgreSQL.
/// For MySQL/SQLite the original `?` form is returned unchanged.
pub(crate) fn rebind(sql: &str, backend: DbBackend) -> String {
    if backend != DbBackend::Postgres {
        return sql.to_string();
    }
    let mut out = String::with_capacity(sql.len() + 16);
    let mut n = 1usize;
    let mut chars = sql.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '?' {
            out.push('$');
            out.push_str(&n.to_string());
            n += 1;
        } else {
            out.push(c);
        }
    }
    out
}
use std::{collections::HashMap, marker::PhantomData};

/// Convert a serde_json Value to a SeaORM bind Value.
fn json_to_sea_value(v: &Value) -> SeaValue {
    match v {
        Value::String(s) => {
            if let Ok(i) = s.parse::<i64>() {
                SeaValue::BigInt(Some(i))
            } else {
                SeaValue::String(Some(Box::new(s.clone())))
            }
        }
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
        // Try integer types first, then float, then String (TEXT)
        let v = if let Ok(i) = i32::try_get_by(row, col.as_str()) {
            json!(i)
        } else if let Ok(i) = i64::try_get_by(row, col.as_str()) {
            json!(i)
        } else if let Ok(f) = f64::try_get_by(row, col.as_str()) {
            json!(f)
        } else if let Ok(b) = bool::try_get_by(row, col.as_str()) {
            json!(b)
        } else if let Ok(s) = String::try_get_by(row, col.as_str()) {
            json!(s)
        } else {
            Value::Null
        };
        map.insert(col.clone(), v);
    }
    map
}

fn is_safe_column_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
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
        let mut where_parts: Vec<String> = Vec::new();

        let search_cols = if !params.search_columns.is_empty() {
            &params.search_columns
        } else {
            &self.search_columns
        };

        if let Some(ref search) = params.search {
            if !search.is_empty() && !search_cols.is_empty() {
                let clauses: Vec<String> =
                    search_cols.iter().map(|c| format!("{} LIKE ?", c)).collect();
                for _ in search_cols {
                    bind_vals.push(SeaValue::String(Some(Box::new(format!("%{}%", search)))));
                }
                where_parts.push(format!("({})", clauses.join(" OR ")));
            }
        }

        let mut filter_cols: Vec<String> = params.filters.keys().cloned().collect();
        filter_cols.sort(); // deterministic order for tests
        for col in &filter_cols {
            if !is_safe_column_name(col) {
                continue;
            }
            if let Some(val) = params.filters.get(col) {
                let s = match val {
                    Value::String(s) if !s.is_empty() => s.clone(),
                    Value::Number(n) => n.to_string(),
                    _ => continue,
                };
                where_parts.push(format!("{} = ?", col));
                bind_vals.push(SeaValue::String(Some(Box::new(s))));
            }
        }

        let where_clause = if where_parts.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_parts.join(" AND "))
        };

        let order_clause = match &params.order_by {
            Some((col, crate::adapter::SortOrder::Desc)) => format!(" ORDER BY {} DESC", col),
            Some((col, crate::adapter::SortOrder::Asc)) => format!(" ORDER BY {} ASC", col),
            None => String::new(),
        };

        let offset = params.page.saturating_sub(1) * params.per_page;
        let sql = rebind(
            &format!("SELECT * FROM {}{}{} LIMIT ? OFFSET ?", table, where_clause, order_clause),
            self.db.get_database_backend(),
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
        let sql = rebind(&format!("SELECT * FROM {} WHERE id = ? LIMIT 1", table), self.db.get_database_backend());
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
        if cols.is_empty() {
            return Err(AdminError::DatabaseError("No fields provided for insert".to_string()));
        }
        let placeholders = cols.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let vals: Vec<SeaValue> = cols
            .iter()
            .map(|k| json_to_sea_value(data.get(k).unwrap()))
            .collect();
        let backend = self.db.get_database_backend();
        if backend == DbBackend::Postgres {
            let raw = format!(
                "INSERT INTO {} ({}) VALUES ({}) RETURNING id",
                table,
                cols.join(", "),
                placeholders,
            );
            let sql = rebind(&raw, backend);
            let stmt = Statement::from_sql_and_values(backend, &sql, vals);
            let row = self
                .db
                .query_one(stmt)
                .await
                .map_err(|e| AdminError::DatabaseError(e.to_string()))?
                .ok_or_else(|| AdminError::DatabaseError("INSERT returned no row".to_string()))?;
            let id = i32::try_get_by(&row, "id")
                .map(|n| n as i64)
                .or_else(|_| i64::try_get_by(&row, "id"))
                .map_err(|_| AdminError::DatabaseError("failed to read inserted id".to_string()))?;
            Ok(Value::Number(id.into()))
        } else {
            let raw = format!(
                "INSERT INTO {} ({}) VALUES ({})",
                table,
                cols.join(", "),
                placeholders,
            );
            let sql = rebind(&raw, backend);
            let stmt = Statement::from_sql_and_values(backend, &sql, vals);
            let res = self
                .db
                .execute(stmt)
                .await
                .map_err(|e| AdminError::DatabaseError(e.to_string()))?;
            Ok(Value::Number(res.last_insert_id().into()))
        }
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
        let sql = rebind(&format!("UPDATE {} SET {} WHERE id = ?", table, set_clause), self.db.get_database_backend());
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
        let sql = rebind(&format!("DELETE FROM {} WHERE id = ?", table), self.db.get_database_backend());
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

        let mut filter_cols: Vec<String> = params.filters.keys().cloned().collect();
        filter_cols.sort();
        for col in &filter_cols {
            if !is_safe_column_name(col) {
                continue;
            }
            if let Some(val) = params.filters.get(col) {
                let s = match val {
                    Value::String(s) if !s.is_empty() => s.clone(),
                    Value::Number(n) => n.to_string(),
                    _ => continue,
                };
                query = query.filter(
                    Expr::col(sea_orm::sea_query::Alias::new(col.as_str())).eq(s)
                );
            }
        }

        query
            .count(&self.db)
            .await
            .map_err(|e| AdminError::DatabaseError(e.to_string()))
    }
}

use crate::adapter::ManyToManyAdapter;
use crate::field::{Field, FieldType};

/// SeaORM-backed adapter for ManyToMany fields.
///
/// Uses raw SQL against a junction table — no SeaORM entity definition required
/// for the junction table itself.
///
/// # Example
/// ```ignore
/// Field::many_to_many(
///     "tags",
///     Box::new(SeaOrmManyToManyAdapter::new(
///         db.clone(),
///         "post_tags",  // junction table
///         "post_id",    // column referencing the current entity
///         "tag_id",     // column referencing the related entity
///         "tags",       // related entity table (for options)
///         "id",         // value column on related table
///         "name",       // label column on related table
///     )),
/// )
/// ```
pub struct SeaOrmManyToManyAdapter {
    db: DatabaseConnection,
    junction_table: String,
    source_col: String,
    target_col: String,
    options_table: String,
    value_col: String,
    label_col: String,
}

impl SeaOrmManyToManyAdapter {
    pub fn new(
        db: DatabaseConnection,
        junction_table: &str,
        source_col: &str,
        target_col: &str,
        options_table: &str,
        value_col: &str,
        label_col: &str,
    ) -> Self {
        Self {
            db,
            junction_table: junction_table.to_string(),
            source_col: source_col.to_string(),
            target_col: target_col.to_string(),
            options_table: options_table.to_string(),
            value_col: value_col.to_string(),
            label_col: label_col.to_string(),
        }
    }
}

#[async_trait]
impl ManyToManyAdapter for SeaOrmManyToManyAdapter {
    async fn fetch_options(&self) -> Result<Vec<(String, String)>, AdminError> {
        let sql = format!(
            "SELECT {}, {} FROM {} ORDER BY {}",
            self.value_col, self.label_col, self.options_table, self.label_col
        );
        let stmt = Statement::from_string(self.db.get_database_backend(), sql);
        let rows = self
            .db
            .query_all(stmt)
            .await
            .map_err(|e| AdminError::DatabaseError(e.to_string()))?;

        Ok(rows
            .iter()
            .filter_map(|row| {
                let value = String::try_get_by(row, self.value_col.as_str())
                    .ok()
                    .or_else(|| i32::try_get_by(row, self.value_col.as_str()).ok().map(|n| n.to_string()))
                    .or_else(|| i64::try_get_by(row, self.value_col.as_str()).ok().map(|n| n.to_string()))?;
                let label = String::try_get_by(row, self.label_col.as_str())
                    .unwrap_or_else(|_| value.clone());
                Some((value, label))
            })
            .collect())
    }

    async fn fetch_selected(&self, record_id: &Value) -> Result<Vec<String>, AdminError> {
        let id_val = json_to_sea_value(record_id);
        let sql = rebind(
            &format!("SELECT {} FROM {} WHERE {} = ?", self.target_col, self.junction_table, self.source_col),
            self.db.get_database_backend(),
        );
        let stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            &sql,
            [id_val],
        );
        let rows = self
            .db
            .query_all(stmt)
            .await
            .map_err(|e| AdminError::DatabaseError(e.to_string()))?;

        Ok(rows
            .iter()
            .filter_map(|row| {
                String::try_get_by(row, self.target_col.as_str())
                    .ok()
                    .or_else(|| i32::try_get_by(row, self.target_col.as_str()).ok().map(|n| n.to_string()))
                    .or_else(|| i64::try_get_by(row, self.target_col.as_str()).ok().map(|n| n.to_string()))
            })
            .collect())
    }

    async fn save(&self, record_id: &Value, selected_ids: Vec<String>) -> Result<(), AdminError> {
        let id_val = json_to_sea_value(record_id);

        // Delete existing junction rows for this record
        let del_sql = rebind(
            &format!("DELETE FROM {} WHERE {} = ?", self.junction_table, self.source_col),
            self.db.get_database_backend(),
        );
        let del_stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            &del_sql,
            [id_val.clone()],
        );
        self.db
            .execute(del_stmt)
            .await
            .map_err(|e| AdminError::DatabaseError(e.to_string()))?;

        // Insert new rows
        for target_id in &selected_ids {
            let ins_sql = rebind(
                &format!("INSERT INTO {} ({}, {}) VALUES (?, ?)", self.junction_table, self.source_col, self.target_col),
                self.db.get_database_backend(),
            );
            let ins_stmt = Statement::from_sql_and_values(
                self.db.get_database_backend(),
                &ins_sql,
                [
                    id_val.clone(),
                    if let Ok(n) = target_id.parse::<i64>() {
                        SeaValue::BigInt(Some(n))
                    } else {
                        SeaValue::String(Some(Box::new(target_id.clone())))
                    },
                ],
            );
            self.db
                .execute(ins_stmt)
                .await
                .map_err(|e| AdminError::DatabaseError(e.to_string()))?;
        }

        Ok(())
    }
}

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
                f = f.readonly().list_only();
            }
            f
        })
        .collect()
}
