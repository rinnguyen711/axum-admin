use crate::{
    app::{AdminApp, AdminAppState},
    auth::AdminAuth,
    middleware::{require_auth, SESSION_COOKIE},
    render::context::{
        ActionContext as ActionCtx, EntityRef, FieldContext, FormContext, ListContext, LoginContext,
        RowContext,
    },
};
use axum::{
    extract::{Extension, Form, Path, Query, RawQuery},
    http::{header, header::LOCATION, StatusCode},
    middleware,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{delete, get, post},
    Router,
};
use form_urlencoded;
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tower_cookies::{Cookie, CookieManagerLayer, Cookies};

const CSRF_COOKIE: &str = "axum_admin_csrf";

fn generate_csrf_token() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Returns the current CSRF token from the cookie, creating one if absent.
fn get_or_create_csrf(cookies: &Cookies) -> String {
    if let Some(c) = cookies.get(CSRF_COOKIE) {
        return c.value().to_string();
    }
    let token = generate_csrf_token();
    let mut cookie = Cookie::new(CSRF_COOKIE, token.clone());
    cookie.set_http_only(true);
    cookie.set_path("/admin");
    cookies.add(cookie);
    token
}

/// Validates the CSRF token from a submitted form against the cookie.
/// Returns `true` if valid.
fn validate_csrf(cookies: &Cookies, form_token: Option<&str>) -> bool {
    match (cookies.get(CSRF_COOKIE), form_token) {
        (Some(cookie), Some(form)) => !form.is_empty() && cookie.value() == form,
        _ => false,
    }
}

// --- Query params ---
#[derive(Deserialize, Default)]
struct ListQuery {
    page: Option<u64>,
    search: Option<String>,
    order_by: Option<String>,
    order_dir: Option<String>,
}

// --- Helpers ---
fn entity_refs(state: &AdminAppState) -> Vec<EntityRef> {
    state
        .entities
        .iter()
        .map(|e| EntityRef {
            name: e.entity_name.clone(),
            label: e.label.clone(),
        })
        .collect()
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn row_to_context(row: &HashMap<String, Value>) -> RowContext {
    let id = row.get("id").map(value_to_string).unwrap_or_default();
    let data = row
        .iter()
        .map(|(k, v)| (k.clone(), value_to_string(v)))
        .collect();
    RowContext { id, data }
}

fn parse_filters(raw_query: Option<&str>) -> HashMap<String, Value> {
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

fn resolve_filter_fields<'a>(entity: &'a crate::entity::EntityAdmin) -> Vec<&'a crate::field::Field> {
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

fn filter_fields_to_context(fields: &[&crate::field::Field]) -> Vec<FieldContext> {
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
        }
    }).collect()
}

async fn fields_to_context(fields: &[crate::field::Field]) -> Vec<FieldContext> {
    use crate::field::FieldType;
    let mut result = Vec::with_capacity(fields.len());
    for f in fields {
        let (type_str, options) = match &f.field_type {
            FieldType::Text => ("Text".to_string(), vec![]),
            FieldType::TextArea => ("TextArea".to_string(), vec![]),
            FieldType::Email => ("Email".to_string(), vec![]),
            FieldType::Password => ("Password".to_string(), vec![]),
            FieldType::Number => ("Number".to_string(), vec![]),
            FieldType::Float => ("Float".to_string(), vec![]),
            FieldType::Boolean => ("Boolean".to_string(), vec![]),
            FieldType::Date => ("Date".to_string(), vec![]),
            FieldType::DateTime => ("DateTime".to_string(), vec![]),
            FieldType::Json => ("Json".to_string(), vec![]),
            FieldType::Select(opts) => ("Select".to_string(), opts.clone()),
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
                ("Select".to_string(), options)
            }
            FieldType::Custom(_) => ("Text".to_string(), vec![]),
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
        });
    }
    result
}

#[allow(clippy::too_many_arguments)]
async fn render_form_error(
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
    let values = form
        .into_iter()
        .map(|(k, v)| (k, value_to_string(&v)))
        .collect();
    let ctx = FormContext {
        admin_title: state.title.clone(),
        entities: entity_refs(state),
        current_entity: entity_name.to_string(),
        entity_name: entity_name.to_string(),
        entity_label: entity.label.clone(),
        fields: fields_to_context(&entity.fields).await,
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

// --- Static assets ---
async fn serve_htmx() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/javascript")],
        include_str!("../static/htmx.min.js"),
    )
}

async fn serve_alpine() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/javascript")],
        include_str!("../static/alpine.min.js"),
    )
}

async fn serve_admin_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css")],
        include_str!("../static/admin.css"),
    )
}

// --- Login ---
async fn login_page(
    cookies: Cookies,
    Query(query): Query<LoginQuery>,
    Extension(state): Extension<Arc<AdminAppState>>,
) -> Html<String> {
    let csrf_token = get_or_create_csrf(&cookies);
    let ctx = LoginContext {
        admin_title: state.title.clone(),
        error: None,
        csrf_token,
        next: query.next,
    };
    Html(state.renderer.render("login.html", ctx))
}

#[derive(Deserialize)]
struct LoginQuery {
    next: Option<String>,
}

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
    next: Option<String>,
}

async fn login_submit(
    cookies: Cookies,
    Extension(auth): Extension<Arc<dyn AdminAuth>>,
    Extension(state): Extension<Arc<AdminAppState>>,
    Form(form): Form<LoginForm>,
) -> Response {
    let next = form.next.clone().filter(|s| s.starts_with('/'));
    match auth.authenticate(&form.username, &form.password).await {
        Ok(user) => {
            // Rotate CSRF token on login
            cookies.remove(Cookie::from(CSRF_COOKIE));
            cookies.add(Cookie::new(SESSION_COOKIE, user.session_id));
            let redirect_to = next.unwrap_or_else(|| "/admin/".to_string());
            (StatusCode::FOUND, [(LOCATION, redirect_to)]).into_response()
        }
        Err(_) => {
            let csrf_token = get_or_create_csrf(&cookies);
            let ctx = LoginContext {
                admin_title: state.title.clone(),
                error: Some("Invalid username or password.".to_string()),
                csrf_token,
                next,
            };
            Html(state.renderer.render("login.html", ctx)).into_response()
        }
    }
}

async fn logout(cookies: Cookies) -> Redirect {
    cookies.remove(Cookie::from(SESSION_COOKIE));
    Redirect::to("/admin/login")
}

// --- Dashboard home ---
async fn admin_home(Extension(state): Extension<Arc<AdminAppState>>) -> Html<String> {
    use crate::render::context::EntityRef;
    use serde::Serialize;
    #[derive(Serialize)]
    struct HomeContext {
        admin_title: String,
        entities: Vec<EntityRef>,
        current_entity: String,
        flash_success: Option<String>,
        flash_error: Option<String>,
    }
    let ctx = HomeContext {
        admin_title: state.title.clone(),
        entities: state.entities.iter().map(|e| EntityRef {
            name: e.entity_name.clone(),
            label: e.label.clone(),
        }).collect(),
        current_entity: String::new(),
        flash_success: None,
        flash_error: None,
    };
    Html(state.renderer.render("home.html", ctx))
}

// --- List ---
async fn entity_list(
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
        entities: entity_refs(&state),
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

// --- Create ---
async fn entity_create_form(
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
        entities: entity_refs(&state),
        current_entity: entity_name.clone(),
        entity_name: entity_name.clone(),
        entity_label: entity.label.clone(),
        fields: fields_to_context(&entity.fields).await,
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

async fn entity_create_submit(
    cookies: Cookies,
    Path(entity_name): Path<String>,
    Extension(state): Extension<Arc<AdminAppState>>,
    Form(form): Form<HashMap<String, String>>,
) -> Response {
    if !validate_csrf(&cookies, form.get("csrf_token").map(String::as_str)) {
        return (StatusCode::FORBIDDEN, "Invalid CSRF token").into_response();
    }
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

    let csrf_token = get_or_create_csrf(&cookies);
    let mut data: HashMap<String, Value> = form
        .iter()
        .filter(|(k, _)| k.as_str() != "csrf_token")
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();

    if let Some(hook) = &entity.before_save {
        if let Err(e) = hook(&mut data) {
            return render_form_error(&state, entity, &entity_name, "", data, e, true, csrf_token).await
                .into_response();
        }
    }

    match adapter.create(data).await {
        Ok(_) => Redirect::to(&format!("/admin/{}/", entity_name)).into_response(),
        Err(crate::error::AdminError::ValidationError(errs)) => {
            let ctx = FormContext {
                admin_title: state.title.clone(),
                entities: entity_refs(&state),
                current_entity: entity_name.clone(),
                entity_name: entity_name.clone(),
                entity_label: entity.label.clone(),
                fields: fields_to_context(&entity.fields).await,
                values: form.into_iter().filter(|(k, _)| k != "csrf_token").collect(),
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

// --- Edit ---
async fn entity_edit_form(
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
        entities: entity_refs(&state),
        current_entity: entity_name.clone(),
        entity_name: entity_name.clone(),
        entity_label: entity.label.clone(),
        fields: fields_to_context(&entity.fields).await,
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

async fn entity_edit_submit(
    cookies: Cookies,
    Path((entity_name, id)): Path<(String, String)>,
    Extension(state): Extension<Arc<AdminAppState>>,
    Form(form): Form<HashMap<String, String>>,
) -> Response {
    if !validate_csrf(&cookies, form.get("csrf_token").map(String::as_str)) {
        return (StatusCode::FORBIDDEN, "Invalid CSRF token").into_response();
    }
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

    let csrf_token = get_or_create_csrf(&cookies);
    let mut data: HashMap<String, Value> = form
        .iter()
        .filter(|(k, _)| k.as_str() != "csrf_token")
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();

    if let Some(hook) = &entity.before_save {
        if let Err(e) = hook(&mut data) {
            return render_form_error(&state, entity, &entity_name, &id, data, e, false, csrf_token).await
                .into_response();
        }
    }

    match adapter.update(&Value::String(id.clone()), data).await {
        Ok(_) => Redirect::to(&format!("/admin/{}/", entity_name)).into_response(),
        Err(crate::error::AdminError::ValidationError(errs)) => {
            let ctx = FormContext {
                admin_title: state.title.clone(),
                entities: entity_refs(&state),
                current_entity: entity_name.clone(),
                entity_name: entity_name.clone(),
                entity_label: entity.label.clone(),
                fields: fields_to_context(&entity.fields).await,
                values: form.into_iter().filter(|(k, _)| k != "csrf_token").collect(),
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

// --- Delete ---
async fn entity_delete(
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

// --- Action ---
async fn entity_action(
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

// --- Router assembly ---
impl AdminApp {
    pub fn into_router(self) -> Router {
        let (auth, state) = self.into_state();

        let protected = Router::new()
            .route("/admin", get(|| async { Redirect::permanent("/admin/") }))
            .route("/admin/", get(admin_home))
            .route("/admin/logout", get(logout))
            .route("/admin/:entity", get(|Path(e): Path<String>| async move {
                Redirect::permanent(&format!("/admin/{}/", e))
            }))
            .route("/admin/:entity/", get(entity_list))
            .route("/admin/:entity/new", get(entity_create_form))
            .route("/admin/:entity/new", post(entity_create_submit))
            .route("/admin/:entity/:id/", get(entity_edit_form))
            .route("/admin/:entity/:id/", post(entity_edit_submit))
            .route("/admin/:entity/:id/delete", delete(entity_delete))
            .route("/admin/:entity/action/:action_name", post(entity_action))
            .route_layer(middleware::from_fn(require_auth));

        Router::new()
            .route("/admin/login", get(login_page))
            .route("/admin/login", post(login_submit))
            .route("/admin/_static/htmx.min.js", get(serve_htmx))
            .route("/admin/_static/alpine.min.js", get(serve_alpine))
            .route("/admin/_static/admin.css", get(serve_admin_css))
            .merge(protected)
            .layer(Extension(state))
            .layer(Extension(auth))
            .layer(CookieManagerLayer::new())
    }
}
