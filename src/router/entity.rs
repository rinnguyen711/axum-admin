use crate::{
    app::AdminAppState,
    render::context::{
        ActionContext as ActionCtx, FormContext, ListContext, NavItem,
    },
};
use axum::{
    extract::{Extension, Multipart, Path, Query, RawQuery},
    http::{header, header::LOCATION, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tower_cookies::Cookies;

use super::csrf::{get_or_create_csrf, validate_csrf};
use super::helpers::{
    build_nav, entity_refs, extract_m2m_data, fields_to_context, filter_fields_to_context,
    parse_filters, parse_multipart, render_form_error, resolve_filter_fields, row_to_context,
    save_m2m, validate_fields, validate_fields_async, value_to_string,
};

#[derive(Deserialize, Default)]
pub(super) struct ListQuery {
    pub(super) page: Option<u64>,
    pub(super) search: Option<String>,
    pub(super) order_by: Option<String>,
    pub(super) order_dir: Option<String>,
}

pub(super) async fn admin_home(Extension(state): Extension<Arc<AdminAppState>>) -> Html<String> {
    use crate::render::context::EntityRef;
    use serde::Serialize;
    #[derive(Serialize)]
    struct HomeContext {
        admin_title: String,
        admin_icon: String,
        entities: Vec<EntityRef>,
        nav: Vec<NavItem>,
        current_entity: String,
        flash_success: Option<String>,
        flash_error: Option<String>,
    }
    let ctx = HomeContext {
        admin_title: state.title.clone(),
        admin_icon: state.icon.clone(),
        entities: entity_refs(&state),
        nav: build_nav(&state, ""),
        current_entity: String::new(),
        flash_success: None,
        flash_error: None,
    };
    Html(state.renderer.render("home.html", ctx))
}

pub(super) async fn entity_list(
    Path(entity_name): Path<String>,
    Query(query): Query<ListQuery>,
    RawQuery(raw_query): RawQuery,
    headers: axum::http::HeaderMap,
    Extension(state): Extension<Arc<AdminAppState>>,
) -> Response {
    let is_htmx = headers.contains_key("hx-request");
    let entity = match state.entities.iter().find(|e| e.entity_name == entity_name) {
        Some(e) => e,
        None => {
            return (axum::http::StatusCode::NOT_FOUND, "Entity not found").into_response()
        }
    };
    let adapter = match &entity.adapter {
        Some(a) => a,
        None => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "No adapter configured",
            )
                .into_response()
        }
    };

    let active_filters_raw = parse_filters(raw_query.as_deref());
    let active_filters: HashMap<String, String> = active_filters_raw
        .iter()
        .filter_map(|(k, v)| {
            if let Value::String(s) = v { Some((k.clone(), s.clone())) } else { None }
        })
        .collect();

    let page = query.page.unwrap_or(1).max(1);
    let per_page = 20u64;
    let params = crate::adapter::ListParams {
        page,
        per_page,
        search: query.search.clone(),
        search_columns: if !entity.search_fields.is_empty() {
            entity.search_fields.clone()
        } else if !entity.list_display.is_empty() {
            entity.list_display.clone()
        } else {
            entity.fields.iter().filter(|f| !f.hidden).map(|f| f.name.clone()).collect()
        },
        filters: active_filters_raw,
        order_by: query.order_by.as_ref().map(|o| {
            let dir = if query.order_dir.as_deref() == Some("desc") {
                crate::adapter::SortOrder::Desc
            } else {
                crate::adapter::SortOrder::Asc
            };
            (o.clone(), dir)
        }),
    };

    let rows = adapter.list(params.clone()).await.unwrap_or_default();
    let total = adapter.count(&params).await.unwrap_or(0);
    let total_pages = ((total as f64) / (per_page as f64)).ceil() as u64;

    let columns = if !entity.list_display.is_empty() {
        entity.list_display.clone()
    } else {
        entity
            .fields
            .iter()
            .filter(|f| !f.hidden)
            .map(|f| f.name.clone())
            .collect()
    };

    let filter_field_defs = resolve_filter_fields(entity);
    let filter_fields_ctx = filter_fields_to_context(&filter_field_defs);

    let export_columns: Vec<(String, String)> = columns.iter().map(|c| {
        let label = entity.fields.iter()
            .find(|f| f.name.as_str() == c.as_str())
            .map(|f| f.label.clone())
            .unwrap_or_else(|| crate::field::default_label(c));
        (c.clone(), label)
    }).collect();

    let ctx = ListContext {
        admin_title: state.title.clone(),
        admin_icon: state.icon.clone(),
        entities: entity_refs(&state),
        nav: build_nav(&state, &entity_name),
        current_entity: entity_name.clone(),
        entity_name: entity_name.clone(),
        entity_label: entity.label.clone(),
        columns,
        rows: rows.iter().map(row_to_context).collect(),
        actions: entity
            .actions
            .iter()
            .map(|a| ActionCtx {
                name: a.name.clone(),
                label: a.label.clone(),
                target: match a.target {
                    crate::entity::ActionTarget::List => "list".to_string(),
                    crate::entity::ActionTarget::Detail => "detail".to_string(),
                },
                confirm: a.confirm.clone(),
                icon: a.icon.clone(),
                class: a.class.clone(),
            })
            .collect(),
        search: query.search.unwrap_or_default(),
        page,
        total_pages: total_pages.max(1),
        order_by: query.order_by.unwrap_or_default(),
        order_dir: query.order_dir.unwrap_or_else(|| "asc".to_string()),
        filter_fields: filter_fields_ctx,
        active_filters,
        bulk_delete: entity.bulk_delete,
        bulk_export: entity.bulk_export,
        export_columns,
        flash_success: None,
        flash_error: None,
    };

    let template = if is_htmx { "list_table.html" } else { "list.html" };
    Html(state.renderer.render(template, ctx)).into_response()
}

pub(super) async fn entity_create_form(
    cookies: Cookies,
    Path(entity_name): Path<String>,
    Extension(state): Extension<Arc<AdminAppState>>,
) -> Response {
    let entity = match state.entities.iter().find(|e| e.entity_name == entity_name) {
        Some(e) => e,
        None => return (axum::http::StatusCode::NOT_FOUND, "Not found").into_response(),
    };

    let csrf_token = get_or_create_csrf(&cookies);
    let ctx = FormContext {
        admin_title: state.title.clone(),
        admin_icon: state.icon.clone(),
        entities: entity_refs(&state),
        nav: build_nav(&state, &entity_name),
        current_entity: entity_name.clone(),
        entity_name: entity_name.clone(),
        entity_label: entity.label.clone(),
        fields: fields_to_context(&entity.fields, &HashMap::new(), None).await,
        values: HashMap::new(),
        errors: HashMap::new(),
        is_create: true,
        record_id: String::new(),
        csrf_token,
        flash_success: None,
        flash_error: None,
    };
    Html(state.renderer.render("form.html", ctx)).into_response()
}

/// Process File/Image fields from multipart data: validate MIME, call storage.save,
/// insert resulting URL into `data`. Returns field-level errors for any failures.
async fn process_file_fields(
    entity_fields: &[crate::field::Field],
    multipart_data: &super::helpers::MultipartData,
    data: &mut HashMap<String, Value>,
) -> HashMap<String, String> {
    use crate::field::FieldType;
    let mut errors: HashMap<String, String> = HashMap::new();

    for field in entity_fields {
        match &field.field_type {
            FieldType::File { storage, accept } => {
                if let Some(upload) = multipart_data.files.get(&field.name) {
                    // Validate MIME if accept list is non-empty
                    if !accept.is_empty() {
                        let mime = &upload.content_type;
                        let ok = accept.iter().any(|a| {
                            if a.ends_with("/*") {
                                let prefix = a.trim_end_matches("/*");
                                mime.starts_with(prefix)
                            } else {
                                mime == a
                            }
                        });
                        if !ok {
                            errors.insert(
                                field.name.clone(),
                                format!("Invalid file type. Allowed: {}", accept.join(", ")),
                            );
                            continue;
                        }
                    }
                    match storage.save(&upload.filename, &upload.data).await {
                        Ok(url) => { data.insert(field.name.clone(), Value::String(url)); }
                        Err(e) => { errors.insert(field.name.clone(), e.to_string()); }
                    }
                } else if let Some(Value::String(s)) = multipart_data.fields.get(&format!("{}__clear", field.name)) {
                    if s == "on" {
                        // Best-effort delete existing file
                        if let Some(Value::String(existing)) = data.get(&field.name) {
                            let _ = storage.delete(existing).await;
                        }
                        data.insert(field.name.clone(), Value::Null);
                    }
                }
                // else: no upload + no clear = do nothing (keep existing or leave empty)
            }
            FieldType::Image { storage } => {
                if let Some(upload) = multipart_data.files.get(&field.name) {
                    if !upload.content_type.starts_with("image/") {
                        errors.insert(
                            field.name.clone(),
                            "Invalid file type. Allowed: image/*".to_string(),
                        );
                        continue;
                    }
                    match storage.save(&upload.filename, &upload.data).await {
                        Ok(url) => { data.insert(field.name.clone(), Value::String(url)); }
                        Err(e) => { errors.insert(field.name.clone(), e.to_string()); }
                    }
                } else if let Some(Value::String(s)) = multipart_data.fields.get(&format!("{}__clear", field.name)) {
                    if s == "on" {
                        if let Some(Value::String(existing)) = data.get(&field.name) {
                            let _ = storage.delete(existing).await;
                        }
                        data.insert(field.name.clone(), Value::Null);
                    }
                }
            }
            _ => {}
        }
    }
    errors
}

pub(super) async fn entity_create_submit(
    cookies: Cookies,
    Path(entity_name): Path<String>,
    Extension(state): Extension<Arc<AdminAppState>>,
    multipart: Multipart,
) -> Response {
    let multipart_data = match parse_multipart(multipart).await {
        Ok(d) => d,
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };

    if !validate_csrf(&cookies, multipart_data.fields.get("csrf_token").and_then(|v| {
        if let Value::String(s) = v { Some(s.as_str()) } else { None }
    })) {
        return (StatusCode::FORBIDDEN, "Invalid CSRF token").into_response();
    }

    let entity = match state.entities.iter().find(|e| e.entity_name == entity_name) {
        Some(e) => e,
        None => return (axum::http::StatusCode::NOT_FOUND, "Not found").into_response(),
    };
    let adapter = match &entity.adapter {
        Some(a) => a,
        None => return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "No adapter").into_response(),
    };

    let csrf_token = get_or_create_csrf(&cookies);
    let mut data: HashMap<String, Value> = multipart_data.fields
        .iter()
        .filter(|(k, _)| k.as_str() != "csrf_token")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    // Process file/image fields: validate MIME, save to storage
    let mut field_errors = process_file_fields(&entity.fields, &multipart_data, &mut data).await;

    // Extract ManyToMany fields from data
    let m2m_data = extract_m2m_data(&entity.fields, &mut data);

    // Run declarative field validators (sync first, then async)
    if field_errors.is_empty() {
        field_errors = validate_fields(&entity.fields, &data);
    }
    if field_errors.is_empty() {
        let async_errors = validate_fields_async(&entity.fields, &data, None).await;
        field_errors.extend(async_errors);
    }
    if !field_errors.is_empty() {
        return render_form_error(
            &state, entity, &entity_name, "",
            data, crate::error::AdminError::ValidationError(field_errors), true, csrf_token,
        ).await.into_response();
    }

    if let Some(hook) = &entity.before_save {
        if let Err(e) = hook(&mut data) {
            return render_form_error(&state, entity, &entity_name, "", data, e, true, csrf_token)
                .await.into_response();
        }
    }

    match adapter.create(data).await {
        Ok(new_id) => {
            save_m2m(&entity.fields, &new_id, m2m_data).await;
            Redirect::to(&format!("/admin/{}/", entity_name)).into_response()
        }
        Err(crate::error::AdminError::ValidationError(errs)) => {
            let values: HashMap<String, String> = multipart_data.fields
                .into_iter()
                .filter(|(k, _)| k != "csrf_token")
                .filter_map(|(k, v)| if let Value::String(s) = v { Some((k, s)) } else { None })
                .collect();
            let ctx = FormContext {
                admin_title: state.title.clone(),
                admin_icon: state.icon.clone(),
                entities: entity_refs(&state),
                nav: build_nav(&state, &entity_name),
                current_entity: entity_name.clone(),
                entity_name: entity_name.clone(),
                entity_label: entity.label.clone(),
                fields: fields_to_context(&entity.fields, &values, None).await,
                values,
                errors: errs,
                is_create: true,
                record_id: String::new(),
                csrf_token,
                flash_success: None,
                flash_error: None,
            };
            Html(state.renderer.render("form.html", ctx)).into_response()
        }
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub(super) async fn entity_edit_form(
    cookies: Cookies,
    Path((entity_name, id)): Path<(String, String)>,
    Extension(state): Extension<Arc<AdminAppState>>,
) -> Response {
    let entity = match state.entities.iter().find(|e| e.entity_name == entity_name) {
        Some(e) => e,
        None => return (axum::http::StatusCode::NOT_FOUND, "Not found").into_response(),
    };
    let adapter = match &entity.adapter {
        Some(a) => a,
        None => {
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "No adapter").into_response()
        }
    };

    let record = match adapter.get(&Value::String(id.clone())).await {
        Ok(r) => r,
        Err(crate::error::AdminError::NotFound) => {
            return (axum::http::StatusCode::NOT_FOUND, "Record not found").into_response()
        }
        Err(e) => {
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    };

    let values: HashMap<String, String> = record
        .iter()
        .map(|(k, v)| (k.clone(), value_to_string(v)))
        .collect();

    let csrf_token = get_or_create_csrf(&cookies);
    let ctx = FormContext {
        admin_title: state.title.clone(),
        admin_icon: state.icon.clone(),
        entities: entity_refs(&state),
        nav: build_nav(&state, &entity_name),
        current_entity: entity_name.clone(),
        entity_name: entity_name.clone(),
        entity_label: entity.label.clone(),
        fields: fields_to_context(&entity.fields, &HashMap::new(), Some(&Value::String(id.clone()))).await,
        values,
        errors: HashMap::new(),
        is_create: false,
        record_id: id,
        csrf_token,
        flash_success: None,
        flash_error: None,
    };
    Html(state.renderer.render("form.html", ctx)).into_response()
}

pub(super) async fn entity_edit_submit(
    cookies: Cookies,
    Path((entity_name, id)): Path<(String, String)>,
    Extension(state): Extension<Arc<AdminAppState>>,
    multipart: Multipart,
) -> Response {
    let multipart_data = match parse_multipart(multipart).await {
        Ok(d) => d,
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };

    if !validate_csrf(&cookies, multipart_data.fields.get("csrf_token").and_then(|v| {
        if let Value::String(s) = v { Some(s.as_str()) } else { None }
    })) {
        return (StatusCode::FORBIDDEN, "Invalid CSRF token").into_response();
    }

    let entity = match state.entities.iter().find(|e| e.entity_name == entity_name) {
        Some(e) => e,
        None => return (axum::http::StatusCode::NOT_FOUND, "Not found").into_response(),
    };
    let adapter = match &entity.adapter {
        Some(a) => a,
        None => return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "No adapter").into_response(),
    };

    let csrf_token = get_or_create_csrf(&cookies);
    let mut data: HashMap<String, Value> = multipart_data.fields
        .iter()
        .filter(|(k, _)| k.as_str() != "csrf_token")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    // Process file/image fields
    let mut field_errors = process_file_fields(&entity.fields, &multipart_data, &mut data).await;

    // Extract ManyToMany fields
    let m2m_data = extract_m2m_data(&entity.fields, &mut data);

    // Run declarative validators
    let record_id_val = Value::String(id.clone());
    if field_errors.is_empty() {
        field_errors = validate_fields(&entity.fields, &data);
    }
    if field_errors.is_empty() {
        let async_errors = validate_fields_async(&entity.fields, &data, Some(&record_id_val)).await;
        field_errors.extend(async_errors);
    }
    if !field_errors.is_empty() {
        return render_form_error(
            &state, entity, &entity_name, &id,
            data, crate::error::AdminError::ValidationError(field_errors), false, csrf_token,
        ).await.into_response();
    }

    if let Some(hook) = &entity.before_save {
        if let Err(e) = hook(&mut data) {
            return render_form_error(&state, entity, &entity_name, &id, data, e, false, csrf_token)
                .await.into_response();
        }
    }

    match adapter.update(&record_id_val, data).await {
        Ok(_) => {
            save_m2m(&entity.fields, &record_id_val, m2m_data).await;
            Redirect::to(&format!("/admin/{}/", entity_name)).into_response()
        }
        Err(crate::error::AdminError::ValidationError(errs)) => {
            let values: HashMap<String, String> = multipart_data.fields
                .into_iter()
                .filter(|(k, _)| k != "csrf_token")
                .filter_map(|(k, v)| if let Value::String(s) = v { Some((k, s)) } else { None })
                .collect();
            let ctx = FormContext {
                admin_title: state.title.clone(),
                admin_icon: state.icon.clone(),
                entities: entity_refs(&state),
                nav: build_nav(&state, &entity_name),
                current_entity: entity_name.clone(),
                entity_name: entity_name.clone(),
                entity_label: entity.label.clone(),
                fields: fields_to_context(&entity.fields, &values, Some(&Value::String(id.clone()))).await,
                values,
                errors: errs,
                is_create: false,
                record_id: id,
                csrf_token,
                flash_success: None,
                flash_error: None,
            };
            Html(state.renderer.render("form.html", ctx)).into_response()
        }
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub(super) async fn entity_delete(
    Path((entity_name, id)): Path<(String, String)>,
    headers: axum::http::HeaderMap,
    Extension(state): Extension<Arc<AdminAppState>>,
) -> Response {
    let entity = match state.entities.iter().find(|e| e.entity_name == entity_name) {
        Some(e) => e,
        None => return (axum::http::StatusCode::NOT_FOUND, "Not found").into_response(),
    };
    let adapter = match &entity.adapter {
        Some(a) => a,
        None => {
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "No adapter").into_response()
        }
    };

    let id_val = Value::String(id.clone());
    let is_htmx = headers.contains_key("hx-request");

    match adapter.delete(&id_val).await {
        Ok(_) => {
            if let Some(hook) = &entity.after_delete {
                let _ = hook(&id_val);
            }
            if is_htmx {
                (StatusCode::OK, [("HX-Refresh", "true")], "").into_response()
            } else {
                (
                    StatusCode::FOUND,
                    [(LOCATION, format!("/admin/{}/", entity_name))],
                )
                    .into_response()
            }
        }
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub(super) async fn entity_action(
    Path((entity_name, action_name)): Path<(String, String)>,
    Extension(state): Extension<Arc<AdminAppState>>,
    axum::extract::RawForm(body): axum::extract::RawForm,
) -> Response {
    let entity = match state.entities.iter().find(|e| e.entity_name == entity_name) {
        Some(e) => e,
        None => return (axum::http::StatusCode::NOT_FOUND, "Entity not found").into_response(),
    };

    // Parse repeated form fields manually (serde_urlencoded doesn't support Vec for repeated keys)
    let pairs: Vec<(String, String)> = form_urlencoded::parse(&body)
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    let selected_ids: Vec<String> = pairs.iter()
        .filter(|(k, _)| k == "selected_ids")
        .map(|(_, v)| v.clone())
        .collect();

    // Built-in bulk delete
    if action_name == "__bulk_delete__" {
        if !entity.bulk_delete {
            return (axum::http::StatusCode::FORBIDDEN, "Bulk delete is disabled").into_response();
        }
        let adapter = match &entity.adapter {
            Some(a) => a,
            None => return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "No adapter").into_response(),
        };
        let mut deleted = 0u64;
        for sid in &selected_ids {
            let id_val = Value::String(sid.clone());
            if adapter.delete(&id_val).await.is_ok() {
                if let Some(hook) = &entity.after_delete {
                    let _ = hook(&id_val);
                }
                deleted += 1;
            }
        }
        use crate::render::context::FlashContext;
        let html = state.renderer.render("flash.html", FlashContext {
            success: Some(format!("{} record(s) deleted.", deleted)),
            error: None,
        });
        return (StatusCode::OK, [("HX-Refresh", "true")], Html(html)).into_response();
    }

    // Built-in bulk export
    if action_name == "__bulk_export__" {
        if !entity.bulk_export {
            return (axum::http::StatusCode::FORBIDDEN, "Bulk export is disabled").into_response();
        }
        let adapter = match &entity.adapter {
            Some(a) => a,
            None => return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "No adapter").into_response(),
        };
        let export_fields: Vec<String> = pairs.iter()
            .filter(|(k, _)| k == "export_fields")
            .map(|(_, v)| v.clone())
            .collect();

        let mut csv = String::new();
        // Header row
        csv.push_str(&export_fields.join(","));
        csv.push('\n');

        // Fetch each record and build rows
        for sid in &selected_ids {
            let id_val = Value::String(sid.clone());
            if let Ok(record) = adapter.get(&id_val).await {
                let row: Vec<String> = export_fields.iter().map(|f| {
                    let raw = record.get(f).map(value_to_string).unwrap_or_default();
                    // Quote fields that contain commas, quotes, or newlines
                    if raw.contains(',') || raw.contains('"') || raw.contains('\n') {
                        format!("\"{}\"", raw.replace('"', "\"\""))
                    } else {
                        raw
                    }
                }).collect();
                csv.push_str(&row.join(","));
                csv.push('\n');
            }
        }

        let filename = format!("{}_export.csv", entity_name);
        let disposition = format!("attachment; filename=\"{}\"", filename);
        let mut response = axum::response::Response::new(axum::body::Body::from(csv));
        *response.status_mut() = StatusCode::OK;
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("text/csv; charset=utf-8"),
        );
        response.headers_mut().insert(
            header::CONTENT_DISPOSITION,
            axum::http::HeaderValue::from_str(&disposition).unwrap(),
        );
        return response;
    }

    let action = match entity.actions.iter().find(|a| a.name == action_name) {
        Some(a) => a,
        None => return (axum::http::StatusCode::NOT_FOUND, "Action not found").into_response(),
    };

    let id: Option<String> = pairs.iter()
        .find(|(k, _)| k == "id")
        .map(|(_, v)| v.clone());

    use crate::entity::{ActionContext, ActionTarget};
    let ids: Vec<Value> = match action.target {
        ActionTarget::List => selected_ids.iter()
            .map(|s| Value::String(s.clone()))
            .collect(),
        ActionTarget::Detail => id.iter()
            .map(|s| Value::String(s.clone()))
            .collect(),
    };

    let ctx = ActionContext {
        ids,
        params: HashMap::new(),
    };

    match (action.handler)(ctx).await {
        Ok(crate::entity::ActionResult::Success(msg)) => {
            use crate::render::context::FlashContext;
            let html = state.renderer.render("flash.html", FlashContext {
                success: Some(msg),
                error: None,
            });
            Html(html).into_response()
        }
        Ok(crate::entity::ActionResult::Error(msg)) => {
            use crate::render::context::FlashContext;
            let html = state.renderer.render("flash.html", FlashContext {
                success: None,
                error: Some(msg),
            });
            Html(html).into_response()
        }
        Ok(crate::entity::ActionResult::Redirect(url)) => {
            (StatusCode::FOUND, [(LOCATION, url)]).into_response()
        }
        Err(e) => {
            use crate::render::context::FlashContext;
            let html = state.renderer.render("flash.html", FlashContext {
                success: None,
                error: Some(e.to_string()),
            });
            Html(html).into_response()
        }
    }
}
