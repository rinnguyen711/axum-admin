use crate::{
    app::AdminAppState,
    auth::AdminAuth,
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
