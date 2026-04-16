use crate::app::AdminAppState;
use crate::auth::AdminUser;
use axum::{
    body::Bytes,
    extract::{Extension, Path},
    http::{header::LOCATION, StatusCode},
    response::{Html, IntoResponse, Response},
};
use serde::Serialize;
use std::sync::Arc;
use tower_cookies::Cookies;

use super::csrf::{get_or_create_csrf, validate_csrf};
use super::helpers::build_nav;

#[derive(Serialize)]
struct RoleRow {
    name: String,
    entity_count: usize,
}

#[derive(Serialize)]
struct RoleListContext {
    admin_title: String,
    admin_icon: String,
    nav: Vec<crate::render::context::NavItem>,
    current_entity: String,
    show_auth_nav: bool,
    roles: Vec<RoleRow>,
    flash_error: Option<String>,
    flash_success: Option<String>,
}

#[derive(Serialize)]
struct PermRow {
    entity_name: String,
    entity_label: String,
    view: bool,
    create: bool,
    edit: bool,
    delete: bool,
}

#[derive(Serialize)]
struct RoleFormContext {
    admin_title: String,
    admin_icon: String,
    nav: Vec<crate::render::context::NavItem>,
    current_entity: String,
    show_auth_nav: bool,
    role_name: Option<String>,
    perms: Vec<PermRow>,
    csrf_token: String,
    error: Option<String>,
}


pub(super) async fn role_list(
    Extension(state): Extension<Arc<AdminAppState>>,
    Extension(user): Extension<AdminUser>,
) -> Response {
    #[cfg(feature = "seaorm")]
    if state.seaorm_auth.is_some() {
        if !user.is_superuser {
            return (StatusCode::FORBIDDEN, "Forbidden").into_response();
        }
        return role_list_with_flash(&state, None, None).await;
    }
    (StatusCode::NOT_FOUND, "Role management requires seaorm feature").into_response()
}

pub(super) async fn role_list_with_flash(
    state: &Arc<AdminAppState>,
    flash_error: Option<String>,
    flash_success: Option<String>,
) -> Response {
    #[cfg(feature = "seaorm")]
    if let Some(ref seaorm) = state.seaorm_auth {
        let role_names = seaorm.list_roles();
        let rows: Vec<RoleRow> = role_names
            .into_iter()
            .map(|name| {
                let perms = seaorm.get_role_permissions(&name);
                let entity_count = perms
                    .iter()
                    .map(|(e, _)| e.clone())
                    .collect::<std::collections::HashSet<_>>()
                    .len();
                RoleRow { name, entity_count }
            })
            .collect();
        let ctx = RoleListContext {
            admin_title: state.title.clone(),
            admin_icon: state.icon.clone(),
            nav: build_nav(state, ""),
            current_entity: "__roles".to_string(),
            show_auth_nav: state.show_auth_nav,
            roles: rows,
            flash_error,
            flash_success,
        };
        return Html(state.renderer.render("roles.html", ctx)).into_response();
    }
    (StatusCode::NOT_FOUND, "Role management requires seaorm feature").into_response()
}

fn build_perm_rows(state: &AdminAppState, checked: &[(String, String)]) -> Vec<PermRow> {
    state
        .entities
        .iter()
        .map(|e| {
            let has = |action: &str| {
                checked
                    .iter()
                    .any(|(ent, act)| ent == &e.entity_name && act == action)
            };
            PermRow {
                entity_name: e.entity_name.clone(),
                entity_label: e.label.clone(),
                view: has("view"),
                create: has("create"),
                edit: has("edit"),
                delete: has("delete"),
            }
        })
        .collect()
}

pub(super) async fn role_create_form(
    cookies: Cookies,
    Extension(state): Extension<Arc<AdminAppState>>,
    Extension(user): Extension<AdminUser>,
) -> Response {
    #[cfg(feature = "seaorm")]
    if let Some(ref _seaorm) = state.seaorm_auth {
        if !user.is_superuser {
            return (StatusCode::FORBIDDEN, "Forbidden").into_response();
        }
        let perms = build_perm_rows(&state, &[]);
        let ctx = RoleFormContext {
            admin_title: state.title.clone(),
            admin_icon: state.icon.clone(),
            nav: build_nav(&state, ""),
            current_entity: "__roles".to_string(),
            show_auth_nav: state.show_auth_nav,
            role_name: None,
            perms,
            csrf_token: get_or_create_csrf(&cookies),
            error: None,
        };
        return Html(state.renderer.render("role_form.html", ctx)).into_response();
    }
    (StatusCode::NOT_FOUND, "Role management requires seaorm feature").into_response()
}

pub(super) async fn role_create_submit(
    cookies: Cookies,
    Extension(state): Extension<Arc<AdminAppState>>,
    Extension(user): Extension<AdminUser>,
    body: Bytes,
) -> Response {
    #[cfg(feature = "seaorm")]
    if let Some(ref seaorm) = state.seaorm_auth {
        if !user.is_superuser {
            return (StatusCode::FORBIDDEN, "Forbidden").into_response();
        }
        let pairs: Vec<(String, String)> = form_urlencoded::parse(&body)
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        let csrf_form = pairs.iter().find(|(k, _)| k == "csrf_token").map(|(_, v)| v.as_str());
        if !validate_csrf(&cookies, csrf_form) {
            return (StatusCode::FORBIDDEN, "Invalid CSRF token").into_response();
        }
        let role_name_raw = pairs.iter()
            .find(|(k, _)| k == "name")
            .map(|(_, v)| v.clone())
            .unwrap_or_default();
        let perm_strings: Vec<String> = pairs.iter()
            .filter(|(k, _)| k == "perms")
            .map(|(_, v)| v.clone())
            .collect();
        let trimmed = role_name_raw.trim();
        let name = if trimmed.is_empty() {
            let perms = build_perm_rows(&state, &[]);
            let ctx = RoleFormContext {
                admin_title: state.title.clone(),
                admin_icon: state.icon.clone(),
                nav: build_nav(&state, ""),
                current_entity: "__roles".to_string(),
                show_auth_nav: state.show_auth_nav,
                role_name: None,
                perms,
                csrf_token: get_or_create_csrf(&cookies),
                error: Some("Role name is required".to_string()),
            };
            return Html(state.renderer.render("role_form.html", ctx)).into_response();
        } else {
            trimmed.to_string()
        };
        let permissions: Vec<(String, String)> = perm_strings
            .iter()
            .filter_map(|p| {
                let mut parts = p.splitn(2, '.');
                let entity = parts.next()?.to_string();
                let action = parts.next()?.to_string();
                Some((entity, action))
            })
            .collect();
        match seaorm.create_role(&name, &permissions).await {
            Ok(_) => {
                return (StatusCode::FOUND, [(LOCATION, "/admin/roles/")]).into_response();
            }
            Err(e) => {
                let perms = build_perm_rows(&state, &permissions);
                let ctx = RoleFormContext {
                    admin_title: state.title.clone(),
                    admin_icon: state.icon.clone(),
                    nav: build_nav(&state, ""),
                    current_entity: "__roles".to_string(),
                    show_auth_nav: state.show_auth_nav,
                    role_name: Some(name),
                    perms,
                    csrf_token: get_or_create_csrf(&cookies),
                    error: Some(e.to_string()),
                };
                return Html(state.renderer.render("role_form.html", ctx)).into_response();
            }
        }
    }
    (StatusCode::NOT_FOUND, "Role management requires seaorm feature").into_response()
}

pub(super) async fn role_edit_form(
    cookies: Cookies,
    Path(role): Path<String>,
    Extension(state): Extension<Arc<AdminAppState>>,
    Extension(user): Extension<AdminUser>,
) -> Response {
    #[cfg(feature = "seaorm")]
    if let Some(ref seaorm) = state.seaorm_auth {
        if !user.is_superuser {
            return (StatusCode::FORBIDDEN, "Forbidden").into_response();
        }
        let current_perms = seaorm.get_role_permissions(&role);
        let perms = build_perm_rows(&state, &current_perms);
        let ctx = RoleFormContext {
            admin_title: state.title.clone(),
            admin_icon: state.icon.clone(),
            nav: build_nav(&state, ""),
            current_entity: "__roles".to_string(),
            show_auth_nav: state.show_auth_nav,
            role_name: Some(role),
            perms,
            csrf_token: get_or_create_csrf(&cookies),
            error: None,
        };
        return Html(state.renderer.render("role_edit_form.html", ctx)).into_response();
    }
    (StatusCode::NOT_FOUND, "Role management requires seaorm feature").into_response()
}

pub(super) async fn role_edit_submit(
    cookies: Cookies,
    Path(role): Path<String>,
    Extension(state): Extension<Arc<AdminAppState>>,
    Extension(user): Extension<AdminUser>,
    body: Bytes,
) -> Response {
    #[cfg(feature = "seaorm")]
    if let Some(ref seaorm) = state.seaorm_auth {
        if !user.is_superuser {
            return (StatusCode::FORBIDDEN, "Forbidden").into_response();
        }
        let pairs: Vec<(String, String)> = form_urlencoded::parse(&body)
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        let csrf_form = pairs.iter().find(|(k, _)| k == "csrf_token").map(|(_, v)| v.as_str());
        if !validate_csrf(&cookies, csrf_form) {
            return (StatusCode::FORBIDDEN, "Invalid CSRF token").into_response();
        }
        let perm_strings: Vec<String> = pairs.iter()
            .filter(|(k, _)| k == "perms")
            .map(|(_, v)| v.clone())
            .collect();
        let permissions: Vec<(String, String)> = perm_strings
            .iter()
            .filter_map(|p| {
                let mut parts = p.splitn(2, '.');
                let entity = parts.next()?.to_string();
                let action = parts.next()?.to_string();
                Some((entity, action))
            })
            .collect();
        match seaorm.update_role_permissions(&role, &permissions).await {
            Ok(_) => {
                return (StatusCode::FOUND, [(LOCATION, "/admin/roles/")]).into_response();
            }
            Err(e) => {
                let perms = build_perm_rows(&state, &permissions);
                let ctx = RoleFormContext {
                    admin_title: state.title.clone(),
                    admin_icon: state.icon.clone(),
                    nav: build_nav(&state, ""),
                    current_entity: "__roles".to_string(),
                    show_auth_nav: state.show_auth_nav,
                    role_name: Some(role),
                    perms,
                    csrf_token: get_or_create_csrf(&cookies),
                    error: Some(e.to_string()),
                };
                return Html(state.renderer.render("role_edit_form.html", ctx)).into_response();
            }
        }
    }
    (StatusCode::NOT_FOUND, "Role management requires seaorm feature").into_response()
}

pub(super) async fn role_delete(
    Path(role): Path<String>,
    Extension(state): Extension<Arc<AdminAppState>>,
    Extension(user): Extension<AdminUser>,
) -> Response {
    #[cfg(feature = "seaorm")]
    if let Some(ref seaorm) = state.seaorm_auth {
        if !user.is_superuser {
            return (StatusCode::FORBIDDEN, "Forbidden").into_response();
        }
        match seaorm.delete_role(&role).await {
            Ok(_) => {
                return role_list_with_flash(&state, None, Some(format!("Role '{}' deleted.", role))).await;
            }
            Err(e) => {
                return role_list_with_flash(&state, Some(e.to_string()), None).await;
            }
        }
    }
    (StatusCode::NOT_FOUND, "Role management requires seaorm feature").into_response()
}
