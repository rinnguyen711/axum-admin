use crate::{
    app::AdminApp,
    auth::AdminAuth,
    middleware::{require_auth, SESSION_COOKIE},
};
use axum::{
    extract::{Extension, Form},
    http::{header::LOCATION, StatusCode},
    middleware,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use std::sync::Arc;
use tower_cookies::{Cookie, CookieManagerLayer, Cookies};

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

async fn login_page() -> Html<&'static str> {
    Html(r#"<!DOCTYPE html>
<html><head><title>Admin Login</title></head>
<body>
<form method="post" action="/admin/login">
  <label>Username <input name="username" type="text" /></label>
  <label>Password <input name="password" type="password" /></label>
  <button type="submit">Login</button>
</form>
</body></html>"#)
}

async fn login_page_with_error() -> Html<&'static str> {
    Html(r#"<!DOCTYPE html>
<html><head><title>Admin Login</title></head>
<body>
<p style="color:red">Invalid username or password.</p>
<form method="post" action="/admin/login">
  <label>Username <input name="username" type="text" /></label>
  <label>Password <input name="password" type="password" /></label>
  <button type="submit">Login</button>
</form>
</body></html>"#)
}

async fn login_submit(
    cookies: Cookies,
    Extension(auth): Extension<Arc<dyn AdminAuth>>,
    Form(form): Form<LoginForm>,
) -> Response {
    match auth.authenticate(&form.username, &form.password).await {
        Ok(user) => {
            cookies.add(Cookie::new(SESSION_COOKIE, user.session_id));
            (StatusCode::FOUND, [(LOCATION, "/admin/")]).into_response()
        }
        Err(_) => login_page_with_error().await.into_response(),
    }
}

async fn logout(cookies: Cookies) -> Redirect {
    cookies.remove(Cookie::from(SESSION_COOKIE));
    Redirect::to("/admin/login")
}

async fn admin_home() -> Html<&'static str> {
    Html("<h1>Admin Dashboard</h1>")
}

impl AdminApp {
    pub fn into_router(self) -> Router {
        let auth: Arc<dyn AdminAuth> = self
            .auth
            .expect("AdminApp requires .auth() to be configured before calling into_router()");

        let protected = Router::new()
            .route("/admin/", get(admin_home))
            .route("/admin/logout", get(logout))
            .route_layer(middleware::from_fn(require_auth));

        Router::new()
            .route("/admin/login", get(login_page))
            .route("/admin/login", post(login_submit))
            .merge(protected)
            .layer(Extension(auth))
            .layer(CookieManagerLayer::new())
    }
}
