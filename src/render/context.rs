use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
pub struct EntityRef {
    pub name: String,
    pub label: String,
}

#[derive(Serialize)]
pub struct FieldContext {
    pub name: String,
    pub label: String,
    pub field_type: String,
    pub readonly: bool,
    pub hidden: bool,
    pub list_only: bool,
    pub form_only: bool,
    pub required: bool,
    pub help_text: Option<String>,
    pub options: Vec<(String, String)>,
}

#[derive(Serialize)]
pub struct RowContext {
    pub id: String,
    pub data: HashMap<String, String>,
}

#[derive(Serialize)]
pub struct ActionContext {
    pub name: String,
    pub label: String,
    pub target: String,
    pub confirm: Option<String>,
    pub icon: Option<String>,
    pub class: Option<String>,
}

#[derive(Serialize)]
pub struct ListContext {
    pub admin_title: String,
    pub entities: Vec<EntityRef>,
    pub current_entity: String,
    pub entity_name: String,
    pub entity_label: String,
    pub columns: Vec<String>,
    pub rows: Vec<RowContext>,
    pub actions: Vec<ActionContext>,
    pub search: String,
    pub page: u64,
    pub total_pages: u64,
    pub order_by: String,
    pub order_dir: String,
    pub filter_fields: Vec<FieldContext>,
    pub active_filters: HashMap<String, String>,
    pub bulk_delete: bool,
    pub bulk_export: bool,
    pub export_columns: Vec<(String, String)>,  // (name, label)
    pub flash_success: Option<String>,
    pub flash_error: Option<String>,
}

#[derive(Serialize)]
pub struct FormContext {
    pub admin_title: String,
    pub entities: Vec<EntityRef>,
    pub current_entity: String,
    pub entity_name: String,
    pub entity_label: String,
    pub fields: Vec<FieldContext>,
    pub values: HashMap<String, String>,
    pub errors: HashMap<String, String>,
    pub is_create: bool,
    pub record_id: String,
    pub csrf_token: String,
    pub flash_success: Option<String>,
    pub flash_error: Option<String>,
}

#[derive(Serialize)]
pub struct LoginContext {
    pub admin_title: String,
    pub error: Option<String>,
    pub csrf_token: String,
    pub next: Option<String>,
}

#[derive(Serialize)]
pub struct FlashContext {
    pub success: Option<String>,
    pub error: Option<String>,
}
