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
    Extension(state): Extension<Arc<AdminAppState>>,
) -> Html<String> {
    let csrf_token = get_or_create_csrf(&cookies);
    let ctx = LoginContext {
        admin_title: state.title.clone(),
        error: None,
        csrf_token,
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
            // Rotate CSRF token on login
            cookies.remove(Cookie::from(CSRF_COOKIE));
            cookies.add(Cookie::new(SESSION_COOKIE, user.session_id));
            (StatusCode::FOUND, [(LOCATION, "/admin/")]).into_response()
        }
        Err(_) => {
            let csrf_token = get_or_create_csrf(&cookies);
            let ctx = LoginContext {
                admin_title: state.title.clone(),
                error: Some("Invalid username or password.".to_string()),
                csrf_token,
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
        search_columns: entity.search_fields.clone(),
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

    match adapter.delete(&id_val).await {
        Ok(_) => {
            if let Some(hook) = &entity.after_delete {
                let _ = hook(&id_val);
            }
            (
                StatusCode::FOUND,
                [(LOCATION, format!("/admin/{}/", entity_name))],
            )
                .into_response()
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

    let action = match entity.actions.iter().find(|a| a.name == action_name) {
        Some(a) => a,
        None => return (axum::http::StatusCode::NOT_FOUND, "Action not found").into_response(),
    };

    // Parse repeated form fields manually (serde_urlencoded doesn't support Vec for repeated keys)
    let pairs: Vec<(String, String)> = form_urlencoded::parse(&body)
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    let selected_ids: Vec<String> = pairs.iter()
        .filter(|(k, _)| k == "selected_ids")
        .map(|(_, v)| v.clone())
        .collect();
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
            .route("/admin/", get(admin_home))
            .route("/admin/logout", get(logout))
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
