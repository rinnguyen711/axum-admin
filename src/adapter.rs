use crate::error::AdminError;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

#[derive(Debug, Clone)]
pub struct ListParams {
    pub page: u64,
    pub per_page: u64,
    pub search: Option<String>,
    pub search_columns: Vec<String>,
    pub filters: HashMap<String, Value>,
    pub order_by: Option<(String, SortOrder)>,
}

impl Default for ListParams {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: 20,
            search: None,
            search_columns: Vec::new(),
            filters: HashMap::new(),
            order_by: None,
        }
    }
}

#[async_trait]
pub trait DataAdapter: Send + Sync {
    async fn list(&self, params: ListParams) -> Result<Vec<HashMap<String, Value>>, AdminError>;
    async fn get(&self, id: &Value) -> Result<HashMap<String, Value>, AdminError>;
    async fn create(&self, data: HashMap<String, Value>) -> Result<Value, AdminError>;
    async fn update(&self, id: &Value, data: HashMap<String, Value>) -> Result<(), AdminError>;
    async fn delete(&self, id: &Value) -> Result<(), AdminError>;
    async fn count(&self, params: &ListParams) -> Result<u64, AdminError>;
}
