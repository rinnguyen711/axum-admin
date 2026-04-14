use crate::{
    app::AdminAppState,
    auth::AdminUser,
};
use axum::{
    extract::{Extension, Form, Multipart, Path, Query, RawQuery},
    http::{header::LOCATION, StatusCode},
    http::HeaderMap,
    response::{Html, IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_cookies::Cookies;

use super::csrf::get_or_create_csrf;
use super::helpers::build_nav;

#[derive(Serialize)]
struct UserRow {
    id: String,
    username: String,
    is_active: bool,
    is_superuser: bool,
    role: Option<String>,
    created_at: String,
}

#[derive(Serialize)]
struct UserListContext {
    admin_title: String,
    admin_icon: String,
    nav: Vec<crate::render::context::NavItem>,
    current_entity: String,
    users: Vec<UserRow>,
    flash_success: Option<String>,
    flash_error: Option<String>,
    show_auth_nav: bool,
}

#[derive(Serialize)]
struct UserFormContext {
    admin_title: String,
    admin_icon: String,
    nav: Vec<crate::render::context::NavItem>,
    current_entity: String,
    csrf_token: String,
    error: Option<String>,
    flash_success: Option<String>,
    flash_error: Option<String>,
    show_auth_nav: bool,
}

#[derive(Deserialize)]
pub(super) struct CreateUserForm {
    username: String,
    password: String,
    is_superuser: Option<String>,
    role: Option<String>,
    #[allow(dead_code)]
    csrf_token: Option<String>,
}

pub(super) async fn user_list(
    query: Query<super::entity::ListQuery>,
    raw_query: RawQuery,
    headers: HeaderMap,
    Extension(state): Extension<Arc<AdminAppState>>,
    Extension(user): Extension<AdminUser>,
) -> Response {
    #[cfg(feature = "seaorm")]
    if let Some(ref seaorm) = state.seaorm_auth {
        use crate::adapters::seaorm_auth::AuthUserEntity;
        use sea_orm::EntityTrait;

        let users = AuthUserEntity::find()
            .all(seaorm.db())
            .await
            .unwrap_or_default();

        let rows: Vec<UserRow> = users.iter().map(|u| UserRow {
            id: u.id.clone(),
            username: u.username.clone(),
            is_active: u.is_active,
            is_superuser: u.is_superuser,
            role: if u.is_superuser {
                None
            } else {
                seaorm.get_user_role(&u.username)
            },
            created_at: u.created_at.format("%Y-%m-%d %H:%M").to_string(),
        }).collect();

        let ctx = UserListContext {
            admin_title: state.title.clone(),
            admin_icon: state.icon.clone(),
            nav: build_nav(&state, ""),
            current_entity: "__users".to_string(),
            users: rows,
            flash_success: None,
            flash_error: None,
            show_auth_nav: state.show_auth_nav,
        };
        return Html(state.renderer.render("users_list.html", ctx)).into_response();
    }

    // Fall through to entity handler when seaorm_auth is not configured
    super::entity::entity_list(
        Path("users".to_string()),
        query,
        raw_query,
        headers,
        Extension(state),
        Extension(user),
    ).await
}

pub(super) async fn user_create_form(
    cookies: Cookies,
    Extension(state): Extension<Arc<AdminAppState>>,
    Extension(user): Extension<AdminUser>,
) -> Response {
    #[cfg(feature = "seaorm")]
    if state.seaorm_auth.is_some() {
        let csrf_token = get_or_create_csrf(&cookies);
        let ctx = UserFormContext {
            admin_title: state.title.clone(),
            admin_icon: state.icon.clone(),
            nav: build_nav(&state, ""),
            current_entity: "__users".to_string(),
            csrf_token,
            error: None,
            flash_success: None,
            flash_error: None,
            show_auth_nav: state.show_auth_nav,
        };
        return Html(state.renderer.render("user_form.html", ctx)).into_response();
    }

    // Fall through to entity handler
    super::entity::entity_create_form(
        cookies,
        Path("users".to_string()),
        Extension(state),
        Extension(user),
    ).await
}

pub(super) async fn user_create_submit(
    cookies: Cookies,
    Extension(state): Extension<Arc<AdminAppState>>,
    Extension(_user): Extension<AdminUser>,
    Form(form): Form<CreateUserForm>,
) -> Response {
    #[cfg(feature = "seaorm")]
    if let Some(ref seaorm) = state.seaorm_auth {
        let is_superuser = form.is_superuser.as_deref() == Some("on");
        match seaorm.create_user(&form.username, &form.password, is_superuser).await {
            Ok(_) => {
                if !is_superuser {
                    let role = form.role.as_deref().unwrap_or("viewer");
                    let _ = seaorm.assign_role(&form.username, role).await;
                }
                return (StatusCode::FOUND, [(LOCATION, "/admin/users/")]).into_response();
            }
            Err(e) => {
                let csrf_token = get_or_create_csrf(&cookies);
                let ctx = UserFormContext {
                    admin_title: state.title.clone(),
                    admin_icon: state.icon.clone(),
                    nav: build_nav(&state, ""),
                    current_entity: "__users".to_string(),
                    csrf_token,
                    error: Some(e.to_string()),
                    flash_success: None,
                    flash_error: None,
                    show_auth_nav: state.show_auth_nav,
                };
                return Html(state.renderer.render("user_form.html", ctx)).into_response();
            }
        }
    }

    // Fall through to entity handler — rebuild the multipart-style submission
    // as a regular form. We pass only the fields the entity handler expects.
    // Since the entity create handler uses Multipart, we can't directly call it
    // with a Form. Return NOT_FOUND to be explicit when seaorm is absent.
    #[cfg(not(feature = "seaorm"))]
    let _ = (cookies, user, form);
    (StatusCode::NOT_FOUND, "User management requires seaorm feature").into_response()
}

pub(super) async fn user_delete(
    Path(id): Path<String>,
    headers: HeaderMap,
    Extension(state): Extension<Arc<AdminAppState>>,
    Extension(user): Extension<AdminUser>,
) -> Response {
    #[cfg(feature = "seaorm")]
    if let Some(ref seaorm) = state.seaorm_auth {
        use crate::adapters::seaorm_auth::AuthUserEntity;
        use sea_orm::EntityTrait;
        let _ = AuthUserEntity::delete_by_id(id).exec(seaorm.db()).await;
        return (StatusCode::FOUND, [(LOCATION, "/admin/users/")]).into_response();
    }

    // Fall through to entity delete handler
    super::entity::entity_delete(
        Path(("users".to_string(), id)),
        headers,
        Extension(state),
        Extension(user),
    ).await
}

pub(super) async fn user_edit_form(
    cookies: Cookies,
    Path(id): Path<String>,
    Extension(state): Extension<Arc<AdminAppState>>,
    Extension(user): Extension<AdminUser>,
) -> Response {
    // Always delegate to entity edit handler — user management has no edit UI
    super::entity::entity_edit_form(
        cookies,
        Path(("users".to_string(), id)),
        Extension(state),
        Extension(user),
    ).await
}

pub(super) async fn user_edit_submit(
    cookies: Cookies,
    Path(id): Path<String>,
    Extension(state): Extension<Arc<AdminAppState>>,
    Extension(user): Extension<AdminUser>,
    multipart: Multipart,
) -> Response {
    super::entity::entity_edit_submit(
        cookies,
        Path(("users".to_string(), id)),
        Extension(state),
        Extension(user),
        multipart,
    ).await
}
