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
    extract::{Extension, Form, Path, Query},
    http::{header, header::LOCATION, StatusCode},
    middleware,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{delete, get, post},
    Router,
};
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tower_cookies::{Cookie, CookieManagerLayer, Cookies};

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

fn fields_to_context(fields: &[crate::field::Field]) -> Vec<FieldContext> {
    use crate::field::FieldType;
    fields
        .iter()
        .map(|f| {
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
                FieldType::Relation { .. } => ("Text".to_string(), vec![]),
                FieldType::Custom(_) => ("Text".to_string(), vec![]),
            };
            FieldContext {
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
            }
        })
        .collect()
}

fn render_form_error(
    state: &AdminAppState,
    entity: &crate::entity::EntityAdmin,
    entity_name: &str,
    form: HashMap<String, Value>,
    err: crate::error::AdminError,
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
        fields: fields_to_context(&entity.fields),
        values,
        errors,
        is_create: true,
        record_id: String::new(),
        csrf_token: "todo-csrf".to_string(),
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

async fn serve_pico_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css")],
        include_str!("../static/pico.min.css"),
    )
}

async fn serve_admin_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css")],
        include_str!("../static/admin.css"),
    )
}

// --- Login ---
async fn login_page(Extension(state): Extension<Arc<AdminAppState>>) -> Html<String> {
    let ctx = LoginContext {
        admin_title: state.title.clone(),
        error: None,
        csrf_token: "todo-csrf".to_string(),
    };
    Html(state.renderer.render("login.html", ctx))
}

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

async fn login_submit(
    cookies: Cookies,
    Extension(auth): Extension<Arc<dyn AdminAuth>>,
    Extension(state): Extension<Arc<AdminAppState>>,
    Form(form): Form<LoginForm>,
) -> Response {
    match auth.authenticate(&form.username, &form.password).await {
        Ok(user) => {
            cookies.add(Cookie::new(SESSION_COOKIE, user.session_id));
            (StatusCode::FOUND, [(LOCATION, "/admin/")]).into_response()
        }
        Err(_) => {
            let ctx = LoginContext {
                admin_title: state.title.clone(),
                error: Some("Invalid username or password.".to_string()),
                csrf_token: "todo-csrf".to_string(),
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
    Html(format!(
        r#"<!DOCTYPE html><html><body><h1>{} Admin</h1><ul>{}</ul></body></html>"#,
        state.title,
        state
            .entities
            .iter()
            .map(|e| format!(
                r#"<li><a href="/admin/{0}/">{1}</a></li>"#,
                e.entity_name, e.label
            ))
            .collect::<String>()
    ))
}

// --- List ---
async fn entity_list(
    Path(entity_name): Path<String>,
    Query(query): Query<ListQuery>,
    Extension(state): Extension<Arc<AdminAppState>>,
) -> Response {
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

    let page = query.page.unwrap_or(1).max(1);
    let per_page = 20u64;
    let params = crate::adapter::ListParams {
        page,
        per_page,
        search: query.search.clone(),
        filters: HashMap::new(),
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
            })
            .collect(),
        search: query.search.unwrap_or_default(),
        page,
        total_pages: total_pages.max(1),
        order_by: query.order_by.unwrap_or_default(),
        order_dir: query.order_dir.unwrap_or_else(|| "asc".to_string()),
        flash_success: None,
        flash_error: None,
    };

    Html(state.renderer.render("list.html", ctx)).into_response()
}

// --- Create ---
async fn entity_create_form(
    Path(entity_name): Path<String>,
    Extension(state): Extension<Arc<AdminAppState>>,
) -> Response {
    let entity = match state.entities.iter().find(|e| e.entity_name == entity_name) {
        Some(e) => e,
        None => return (axum::http::StatusCode::NOT_FOUND, "Not found").into_response(),
    };

    let ctx = FormContext {
        admin_title: state.title.clone(),
        entities: entity_refs(&state),
        current_entity: entity_name.clone(),
        entity_name: entity_name.clone(),
        entity_label: entity.label.clone(),
        fields: fields_to_context(&entity.fields),
        values: HashMap::new(),
        errors: HashMap::new(),
        is_create: true,
        record_id: String::new(),
        csrf_token: "todo-csrf".to_string(),
        flash_success: None,
        flash_error: None,
    };
    Html(state.renderer.render("form.html", ctx)).into_response()
}

async fn entity_create_submit(
    Path(entity_name): Path<String>,
    Extension(state): Extension<Arc<AdminAppState>>,
    Form(form): Form<HashMap<String, String>>,
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

    let mut data: HashMap<String, Value> = form
        .iter()
        .filter(|(k, _)| k.as_str() != "csrf_token")
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();

    if let Some(hook) = &entity.before_save {
        if let Err(e) = hook(&mut data) {
            return render_form_error(&state, entity, &entity_name, data, e).into_response();
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
                fields: fields_to_context(&entity.fields),
                values: form.into_iter().collect(),
                errors: errs,
                is_create: true,
                record_id: String::new(),
                csrf_token: "todo-csrf".to_string(),
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

    let ctx = FormContext {
        admin_title: state.title.clone(),
        entities: entity_refs(&state),
        current_entity: entity_name.clone(),
        entity_name: entity_name.clone(),
        entity_label: entity.label.clone(),
        fields: fields_to_context(&entity.fields),
        values,
        errors: HashMap::new(),
        is_create: false,
        record_id: id,
        csrf_token: "todo-csrf".to_string(),
        flash_success: None,
        flash_error: None,
    };
    Html(state.renderer.render("form.html", ctx)).into_response()
}

async fn entity_edit_submit(
    Path((entity_name, id)): Path<(String, String)>,
    Extension(state): Extension<Arc<AdminAppState>>,
    Form(form): Form<HashMap<String, String>>,
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

    let mut data: HashMap<String, Value> = form
        .iter()
        .filter(|(k, _)| k.as_str() != "csrf_token")
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();

    if let Some(hook) = &entity.before_save {
        if let Err(e) = hook(&mut data) {
            return render_form_error(&state, entity, &entity_name, data, e).into_response();
        }
    }

    match adapter.update(&Value::String(id.clone()), data).await {
        Ok(_) => Redirect::to(&format!("/admin/{}/", entity_name)).into_response(),
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// --- Delete ---
async fn entity_delete(
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

    let id_val = Value::String(id.clone());

    if let Some(hook) = &entity.after_delete {
        let _ = hook(&id_val);
    }

    match adapter.delete(&id_val).await {
        Ok(_) => (
            StatusCode::FOUND,
            [(LOCATION, format!("/admin/{}/", entity_name))],
        )
            .into_response(),
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// --- Router assembly ---
impl AdminApp {
    pub fn into_router(self) -> Router {
        let (auth, state) = self.into_state();

        let protected = Router::new()
            .route("/admin/", get(admin_home))
            .route("/admin/logout", get(logout))
            .route("/admin/:entity/", get(entity_list))
            .route("/admin/:entity/new", get(entity_create_form))
            .route("/admin/:entity/new", post(entity_create_submit))
            .route("/admin/:entity/:id/", get(entity_edit_form))
            .route("/admin/:entity/:id/", post(entity_edit_submit))
            .route("/admin/:entity/:id/delete", delete(entity_delete))
            .route_layer(middleware::from_fn(require_auth));

        Router::new()
            .route("/admin/login", get(login_page))
            .route("/admin/login", post(login_submit))
            .route("/admin/_static/htmx.min.js", get(serve_htmx))
            .route("/admin/_static/alpine.min.js", get(serve_alpine))
            .route("/admin/_static/pico.min.css", get(serve_pico_css))
            .route("/admin/_static/admin.css", get(serve_admin_css))
            .merge(protected)
            .layer(Extension(state))
            .layer(Extension(auth))
            .layer(CookieManagerLayer::new())
    }
}
