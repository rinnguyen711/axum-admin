use crate::{
    app::AdminAppState,
    render::context::{EntityRef, FieldContext, FormContext, NavItem, RowContext},
};
use axum::body::Bytes;
use axum::response::Html;
use form_urlencoded;
use serde_json::Value;
use std::collections::HashMap;

pub(super) fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

pub(super) fn row_to_context(row: &HashMap<String, Value>) -> RowContext {
    let id = row.get("id").map(value_to_string).unwrap_or_default();
    let data = row
        .iter()
        .map(|(k, v)| (k.clone(), value_to_string(v)))
        .collect();
    RowContext { id, data }
}

/// Build the sidebar nav structure: ungrouped entities are top-level `NavItem::Entity`;
/// grouped entities are collected into `NavItem::Group` in first-seen order.
/// `current_entity` is used to mark the active group as open.
pub(super) fn build_nav(state: &AdminAppState, current_entity: &str) -> Vec<NavItem> {
    let mut nav: Vec<NavItem> = Vec::new();
    let mut group_indices: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for e in &state.entities {
        let entity_ref = EntityRef {
            name: e.entity_name.clone(),
            label: e.label.clone(),
            icon: e.icon.clone(),
            group: e.group.clone(),
        };
        match &e.group {
            None => nav.push(NavItem::Entity(entity_ref)),
            Some(group_label) => {
                if let Some(&idx) = group_indices.get(group_label) {
                    if let Some(NavItem::Group { entities, active, .. }) = nav.get_mut(idx) {
                        if e.entity_name == current_entity {
                            *active = true;
                        }
                        entities.push(entity_ref);
                    }
                } else {
                    let is_active = e.entity_name == current_entity;
                    group_indices.insert(group_label.clone(), nav.len());
                    nav.push(NavItem::Group {
                        label: group_label.clone(),
                        entities: vec![entity_ref],
                        active: is_active,
                    });
                }
            }
        }
    }
    nav
}

pub(super) fn entity_refs(state: &AdminAppState) -> Vec<EntityRef> {
    state
        .entities
        .iter()
        .map(|e| EntityRef {
            name: e.entity_name.clone(),
            label: e.label.clone(),
            icon: e.icon.clone(),
            group: e.group.clone(),
        })
        .collect()
}

pub(super) fn parse_filters(raw_query: Option<&str>) -> HashMap<String, Value> {
    let mut filters = HashMap::new();
    if let Some(q) = raw_query {
        for (k, v) in form_urlencoded::parse(q.as_bytes()) {
            if let Some(col) = k.strip_prefix("filter[").and_then(|s| s.strip_suffix("]")) {
                if !v.is_empty() {
                    filters.insert(col.to_string(), Value::String(v.into_owned()));
                }
            }
        }
    }
    filters
}

pub(super) fn resolve_filter_fields<'a>(entity: &'a crate::entity::EntityAdmin) -> Vec<&'a crate::field::Field> {
    // Determine the base set of column names to generate filter inputs for.
    // Priority: explicit filter_fields > list_display > all non-hidden fields.
    let base_names: Vec<&str> = if !entity.filter_fields.is_empty() {
        entity.filter_fields.iter().map(|s| s.as_str()).collect()
    } else if !entity.list_display.is_empty() {
        entity.list_display.iter().map(|s| s.as_str()).collect()
    } else {
        entity.fields.iter().filter(|f| !f.hidden).map(|f| f.name.as_str()).collect()
    };

    let mut result: Vec<&crate::field::Field> = base_names
        .iter()
        .filter_map(|name| entity.fields.iter().find(|f| f.name.as_str() == *name))
        .collect();

    // Upsert entity.filters (custom overrides) by name
    for custom in &entity.filters {
        if let Some(pos) = result.iter().position(|f| f.name == custom.name) {
            result[pos] = custom;
        } else {
            result.push(custom);
        }
    }
    result
}

pub(super) fn filter_fields_to_context(fields: &[&crate::field::Field]) -> Vec<FieldContext> {
    use crate::field::FieldType;
    fields.iter().map(|f| {
        let (type_str, options) = match &f.field_type {
            FieldType::Select(opts) => ("Select".to_string(), opts.clone()),
            FieldType::Boolean => ("Boolean".to_string(), vec![
                ("true".to_string(), "Yes".to_string()),
                ("false".to_string(), "No".to_string()),
            ]),
            _ => ("Text".to_string(), vec![]),
        };
        FieldContext {
            name: f.name.clone(),
            label: f.label.clone(),
            field_type: type_str,
            readonly: false,
            hidden: false,
            list_only: false,
            form_only: false,
            required: false,
            help_text: None,
            options,
            selected_ids: vec![],
        }
    }).collect()
}

pub(super) struct FileUpload {
    pub filename: String,
    pub content_type: String,
    pub data: Bytes,
}

pub(super) struct MultipartData {
    pub fields: HashMap<String, Value>,
    pub files: HashMap<String, FileUpload>,
}

pub(super) async fn parse_multipart(
    mut multipart: axum::extract::Multipart,
) -> Result<MultipartData, String> {
    let mut fields = HashMap::new();
    let mut files = HashMap::new();

    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                let name = match field.name() {
                    Some(n) => n.to_string(),
                    None => continue,
                };
                let filename = field.file_name().map(|s| s.to_string());
                let content_type = field
                    .content_type()
                    .map(|ct| ct.to_string())
                    .unwrap_or_default();

                let data = field
                    .bytes()
                    .await
                    .map_err(|e| format!("multipart read error: {e}"))?;

                match filename {
                    Some(fname) if !fname.is_empty() => {
                        // Strip path components from client-provided filename (e.g. "../../../passwd" → "passwd")
                        let safe_filename = std::path::Path::new(&fname)
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unnamed")
                            .to_string();
                        files.insert(name, FileUpload {
                            filename: safe_filename,
                            content_type,
                            data,
                        });
                    }
                    _ => {
                        // text part
                        let text = String::from_utf8_lossy(&data).into_owned();
                        fields.insert(name, Value::String(text));
                    }
                }
            }
            Ok(None) => break,
            Err(e) => return Err(format!("multipart error: {e}")),
        }
    }

    Ok(MultipartData { fields, files })
}

/// Build template context for a list of fields.
///
/// `submitted_values` — form values from a POST resubmission (used to restore
///   ManyToMany selections when re-rendering a form after a validation error).
/// `record_id` — the current record's ID (used to fetch ManyToMany selections
///   from the DB when opening an edit form fresh).
pub(super) async fn fields_to_context(
    fields: &[crate::field::Field],
    submitted_values: &HashMap<String, String>,
    record_id: Option<&Value>,
) -> Vec<FieldContext> {
    use crate::field::FieldType;
    let mut result = Vec::with_capacity(fields.len());
    for f in fields {
        let (type_str, options, selected_ids) = match &f.field_type {
            FieldType::Text => ("Text".to_string(), vec![], vec![]),
            FieldType::TextArea => ("TextArea".to_string(), vec![], vec![]),
            FieldType::Email => ("Email".to_string(), vec![], vec![]),
            FieldType::Password => ("Password".to_string(), vec![], vec![]),
            FieldType::Number => ("Number".to_string(), vec![], vec![]),
            FieldType::Float => ("Float".to_string(), vec![], vec![]),
            FieldType::Boolean => ("Boolean".to_string(), vec![], vec![]),
            FieldType::Date => ("Date".to_string(), vec![], vec![]),
            FieldType::DateTime => ("DateTime".to_string(), vec![], vec![]),
            FieldType::Json => ("Json".to_string(), vec![], vec![]),
            FieldType::Select(opts) => ("Select".to_string(), opts.clone(), vec![]),
            FieldType::ForeignKey { adapter, value_field, label_field, limit, order_by } => {
                let params = crate::adapter::ListParams {
                    per_page: limit.unwrap_or(i64::MAX as u64),
                    order_by: order_by.as_ref().map(|field| (field.clone(), crate::adapter::SortOrder::Asc)),
                    ..Default::default()
                };
                let rows = adapter.list(params).await.unwrap_or_default();
                let options = rows
                    .iter()
                    .filter_map(|row| {
                        let value = row.get(value_field).map(value_to_string)?;
                        let label = row.get(label_field).map(value_to_string).unwrap_or_else(|| value.clone());
                        Some((value, label))
                    })
                    .collect();
                ("Select".to_string(), options, vec![])
            }
            FieldType::ManyToMany { adapter } => {
                let options = adapter.fetch_options().await.unwrap_or_default();
                // Determine selected IDs: prefer submitted form values (error resubmission),
                // fall back to fetching from the DB for the current record.
                let selected_ids = if let Some(json_str) = submitted_values.get(&f.name) {
                    serde_json::from_str::<Vec<String>>(json_str).unwrap_or_default()
                } else if let Some(id) = record_id {
                    adapter.fetch_selected(id).await.unwrap_or_default()
                } else {
                    vec![]
                };
                ("ManyToMany".to_string(), options, selected_ids)
            }
            FieldType::Custom(_) => ("Text".to_string(), vec![], vec![]),
            FieldType::File { .. } => ("File".to_string(), vec![], vec![]),
            FieldType::Image { .. } => ("Image".to_string(), vec![], vec![]),
        };
        result.push(FieldContext {
            name: f.name.clone(),
            label: f.label.clone(),
            field_type: type_str,
            readonly: f.readonly,
            hidden: f.hidden,
            list_only: f.list_only,
            form_only: f.form_only,
            required: f.required,
            help_text: f.help_text.clone(),
            options,
            selected_ids,
        });
    }
    result
}

/// Remove ManyToMany fields from the data map (they aren't DB columns on the main table)
/// and return them as a vec of (field_name, selected_ids).
pub(super) fn extract_m2m_data(
    fields: &[crate::field::Field],
    data: &mut HashMap<String, Value>,
) -> Vec<(String, Vec<String>)> {
    use crate::field::FieldType;
    fields
        .iter()
        .filter(|f| matches!(f.field_type, FieldType::ManyToMany { .. }))
        .map(|f| {
            let ids = data
                .remove(&f.name)
                .and_then(|v| {
                    if let Value::String(s) = v {
                        serde_json::from_str::<Vec<String>>(&s).ok()
                    } else {
                        None
                    }
                })
                .unwrap_or_default();
            (f.name.clone(), ids)
        })
        .collect()
}

/// Save ManyToMany selections to the junction table for each M2M field.
pub(super) async fn save_m2m(
    fields: &[crate::field::Field],
    record_id: &Value,
    m2m_data: Vec<(String, Vec<String>)>,
) {
    use crate::field::FieldType;
    for (field_name, selected_ids) in m2m_data {
        if let Some(field) = fields.iter().find(|f| f.name == field_name) {
            if let FieldType::ManyToMany { adapter } = &field.field_type {
                let _ = adapter.save(record_id, selected_ids).await;
            }
        }
    }
}

/// Run all synchronous validators on the submitted data.
/// Returns a map of field_name -> first error message. Empty = no errors.
pub(super) fn validate_fields(
    fields: &[crate::field::Field],
    data: &HashMap<String, Value>,
) -> HashMap<String, String> {
    let mut errors: HashMap<String, String> = HashMap::new();
    for field in fields {
        if field.validators.is_empty() { continue; }
        let value = data.get(&field.name)
            .and_then(|v| if let Value::String(s) = v { Some(s.as_str()) } else { None })
            .unwrap_or("");
        for validator in &field.validators {
            if let Err(msg) = validator.validate(value) {
                errors.insert(field.name.clone(), msg);
                break; // first error per field only
            }
        }
    }
    errors
}

/// Run all async validators on the submitted data.
/// `record_id` is `Some` on edit (to exclude current record from uniqueness checks).
pub(super) async fn validate_fields_async(
    fields: &[crate::field::Field],
    data: &HashMap<String, Value>,
    record_id: Option<&Value>,
) -> HashMap<String, String> {
    let mut errors: HashMap<String, String> = HashMap::new();
    for field in fields {
        if field.async_validators.is_empty() { continue; }
        if errors.contains_key(&field.name) { continue; }
        let value = data.get(&field.name)
            .and_then(|v| if let Value::String(s) = v { Some(s.as_str()) } else { None })
            .unwrap_or("");
        for validator in &field.async_validators {
            if let Err(msg) = validator.validate(value, record_id).await {
                errors.insert(field.name.clone(), msg);
                break;
            }
        }
    }
    errors
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn render_form_error(
    state: &AdminAppState,
    entity: &crate::entity::EntityAdmin,
    entity_name: &str,
    record_id: &str,
    form: HashMap<String, Value>,
    err: crate::error::AdminError,
    is_create: bool,
    csrf_token: String,
) -> Html<String> {
    let errors = match err {
        crate::error::AdminError::ValidationError(e) => e,
        other => HashMap::from([("__all__".to_string(), other.to_string())]),
    };
    let values: HashMap<String, String> = form
        .into_iter()
        .map(|(k, v)| (k, value_to_string(&v)))
        .collect();
    let rid = if record_id.is_empty() { None } else { Some(Value::String(record_id.to_string())) };
    let ctx = FormContext {
        admin_title: state.title.clone(),
        admin_icon: state.icon.clone(),
        entities: entity_refs(state),
        nav: build_nav(state, entity_name),
        current_entity: entity_name.to_string(),
        entity_name: entity_name.to_string(),
        entity_label: entity.label.clone(),
        fields: fields_to_context(&entity.fields, &values, rid.as_ref()).await,
        values,
        errors,
        is_create,
        record_id: record_id.to_string(),
        csrf_token,
        flash_success: None,
        flash_error: None,
    };
    Html(state.renderer.render("form.html", ctx))
}
