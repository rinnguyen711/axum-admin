# DataAdapter

`DataAdapter` and `ManyToManyAdapter` are the two traits you implement to connect axum-admin to your database. Both are defined in `axum_admin::adapter` and re-exported at the crate root.

---

## DataAdapter Trait

```rust
#[async_trait]
pub trait DataAdapter: Send + Sync {
    async fn list(&self, params: ListParams) -> Result<Vec<HashMap<String, Value>>, AdminError>;
    async fn get(&self, id: &Value) -> Result<HashMap<String, Value>, AdminError>;
    async fn create(&self, data: HashMap<String, Value>) -> Result<Value, AdminError>;
    async fn update(&self, id: &Value, data: HashMap<String, Value>) -> Result<(), AdminError>;
    async fn delete(&self, id: &Value) -> Result<(), AdminError>;
    async fn count(&self, params: &ListParams) -> Result<u64, AdminError>;
}
```

### Methods

| Method | Description |
|---|---|
| `list(params)` | Return a page of records matching the given `ListParams`. Each record is a `HashMap<String, Value>`. |
| `get(id)` | Fetch a single record by primary key. |
| `create(data)` | Insert a new record; return the new record's primary key as a `Value`. |
| `update(id, data)` | Update an existing record by primary key. |
| `delete(id)` | Delete a record by primary key. |
| `count(params)` | Return the total count of records matching the params (used for pagination). |

---

## ListParams

`ListParams` is passed to `list` and `count` to describe the current query.

```rust
pub struct ListParams {
    pub page: u64,
    pub per_page: u64,
    pub search: Option<String>,
    pub search_columns: Vec<String>,
    pub filters: HashMap<String, Value>,
    pub order_by: Option<(String, SortOrder)>,
}
```

| Field | Default | Description |
|---|---|---|
| `page` | `1` | 1-based page number |
| `per_page` | `20` | Rows per page |
| `search` | `None` | Full-text search query string |
| `search_columns` | `[]` | Columns to search against |
| `filters` | `{}` | Exact-match column filters, e.g. `{"status": "active"}` |
| `order_by` | `None` | Column name and `SortOrder` direction |

`ListParams` implements `Default`, so you can use struct-update syntax:

```rust
let params = ListParams {
    page: 2,
    per_page: 50,
    ..Default::default()
};
```

---

## SortOrder Enum

```rust
#[derive(Debug, Clone, Default)]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}
```

The default variant is `Asc`. Used as the second element of the `order_by` tuple in `ListParams`.

---

## ManyToManyAdapter Trait

`ManyToManyAdapter` drives `Field::many_to_many` fields. It manages the options list and the junction-table read/write for a specific relation.

```rust
#[async_trait]
pub trait ManyToManyAdapter: Send + Sync {
    async fn fetch_options(&self) -> Result<Vec<(String, String)>, AdminError>;
    async fn fetch_selected(&self, record_id: &Value) -> Result<Vec<String>, AdminError>;
    async fn save(&self, record_id: &Value, selected_ids: Vec<String>) -> Result<(), AdminError>;
}
```

### Methods

| Method | Description |
|---|---|
| `fetch_options()` | Return all available options as `(value, label)` pairs. |
| `fetch_selected(record_id)` | Return the IDs currently selected for the given record. |
| `save(record_id, selected_ids)` | Atomically replace the current selection (delete + insert in the junction table). |

---

## SeaORM Adapter

When the `seaorm` feature is enabled, axum-admin ships a `SeaOrmAdapter` in `axum_admin::adapters::seaorm`. It implements `DataAdapter` on top of a SeaORM `DatabaseConnection` and a model type.

Enable the feature in `Cargo.toml`:

```toml
[dependencies]
axum-admin = { version = "0.1", features = ["seaorm"] }
```

Refer to the [Quick Start](../quick-start.md) guide and the SeaORM adapter source for usage details. For custom databases or ORMs, implement `DataAdapter` directly.

---

## Example: minimal DataAdapter

```rust
use axum_admin::{DataAdapter, ListParams, AdminError};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

pub struct MyAdapter { /* db connection etc. */ }

#[async_trait]
impl DataAdapter for MyAdapter {
    async fn list(&self, params: ListParams) -> Result<Vec<HashMap<String, Value>>, AdminError> {
        // query your database, apply params.search, params.filters, params.order_by
        // return one page of results
        todo!()
    }

    async fn get(&self, id: &Value) -> Result<HashMap<String, Value>, AdminError> {
        todo!()
    }

    async fn create(&self, data: HashMap<String, Value>) -> Result<Value, AdminError> {
        // insert and return new PK
        todo!()
    }

    async fn update(&self, id: &Value, data: HashMap<String, Value>) -> Result<(), AdminError> {
        todo!()
    }

    async fn delete(&self, id: &Value) -> Result<(), AdminError> {
        todo!()
    }

    async fn count(&self, params: &ListParams) -> Result<u64, AdminError> {
        todo!()
    }
}
```
