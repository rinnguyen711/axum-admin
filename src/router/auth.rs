use crate::{
    app::AdminAppState,
    auth::{AdminAuth, AdminUser},
    middleware::SESSION_COOKIE,
    render::context::LoginContext,
};
use axum::{
    extract::{Extension, Form, Query},
    http::{header::LOCATION, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use std::sync::Arc;
use tower_cookies::{Cookie, Cookies};

use super::csrf::{get_or_create_csrf, CSRF_COOKIE};

#[derive(Deserialize)]
pub(super) struct LoginQuery {
    pub(super) next: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct LoginForm {
    username: String,
    password: String,
    next: Option<String>,
}

pub(super) async fn login_page(
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

pub(super) async fn login_submit(
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

pub(super) async fn logout(cookies: Cookies) -> Redirect {
    cookies.remove(Cookie::from(SESSION_COOKIE));
    Redirect::to("/admin/login")
}

#[derive(serde::Serialize)]
struct ChangePasswordContext {
    admin_title: String,
    admin_icon: String,
    nav: Vec<crate::render::context::NavItem>,
    current_entity: String,
    csrf_token: String,
    error: Option<String>,
    success: Option<String>,
    flash_success: Option<String>,
    flash_error: Option<String>,
}

#[derive(serde::Deserialize)]
pub(super) struct ChangePasswordForm {
    current_password: String,
    new_password: String,
    confirm_password: String,
    #[allow(dead_code)]
    csrf_token: Option<String>,
}

async fn make_change_password_ctx(
    state: &Arc<AdminAppState>,
    user: &AdminUser,
    csrf_token: &str,
    error: Option<String>,
    success: Option<String>,
) -> ChangePasswordContext {
    ChangePasswordContext {
        admin_title: state.title.clone(),
        admin_icon: state.icon.clone(),
        nav: super::helpers::build_nav(
            state,
            "",
            user,
            #[cfg(feature = "seaorm")]
            state.enforcer.as_ref(),
            #[cfg(not(feature = "seaorm"))]
            None,
        ).await,
        current_entity: String::new(),
        csrf_token: csrf_token.to_string(),
        error,
        success,
        flash_success: None,
        flash_error: None,
    }
}

pub(super) async fn change_password_page(
    cookies: Cookies,
    Extension(state): Extension<Arc<AdminAppState>>,
    Extension(user): Extension<AdminUser>,
) -> Html<String> {
    let csrf_token = get_or_create_csrf(&cookies);
    let ctx = make_change_password_ctx(&state, &user, &csrf_token, None, None).await;
    Html(state.renderer.render("change_password.html", ctx))
}

pub(super) async fn change_password_submit(
    cookies: Cookies,
    Extension(state): Extension<Arc<AdminAppState>>,
    Extension(user): Extension<AdminUser>,
    Form(form): Form<ChangePasswordForm>,
) -> Html<String> {
    let csrf_token = get_or_create_csrf(&cookies);

    if form.new_password != form.confirm_password {
        return Html(state.renderer.render(
            "change_password.html",
            make_change_password_ctx(&state, &user, &csrf_token, Some("New passwords do not match.".into()), None).await,
        ));
    }
    if form.new_password.len() < 8 {
        return Html(state.renderer.render(
            "change_password.html",
            make_change_password_ctx(&state, &user, &csrf_token, Some("New password must be at least 8 characters.".into()), None).await,
        ));
    }

    #[cfg(feature = "seaorm")]
    if let Some(ref seaorm) = state.seaorm_auth {
        return match seaorm.change_password(&user.username, &form.current_password, &form.new_password).await {
            Ok(_) => Html(state.renderer.render(
                "change_password.html",
                make_change_password_ctx(&state, &user, &csrf_token, None, Some("Password updated successfully.".into())).await,
            )),
            Err(crate::error::AdminError::Unauthorized) => Html(state.renderer.render(
                "change_password.html",
                make_change_password_ctx(&state, &user, &csrf_token, Some("Current password is incorrect.".into()), None).await,
            )),
            Err(e) => Html(state.renderer.render(
                "change_password.html",
                make_change_password_ctx(&state, &user, &csrf_token, Some(e.to_string()), None).await,
            )),
        };
    }

    Html(state.renderer.render(
        "change_password.html",
        make_change_password_ctx(&state, &user, &csrf_token, Some("Password change is not supported by the current auth backend.".into()), None).await,
    ))
}
