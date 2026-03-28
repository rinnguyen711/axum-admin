use crate::auth::AdminAuth;
use axum::{
    extract::Request,
    http::{header::LOCATION, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Extension,
};
use std::sync::Arc;
use tower_cookies::Cookies;

pub const SESSION_COOKIE: &str = "axum_admin_session";

pub async fn require_auth(
    cookies: Cookies,
    Extension(auth): Extension<Arc<dyn AdminAuth>>,
    req: Request,
    next: Next,
) -> Response {
    let session_id = cookies.get(SESSION_COOKIE).map(|c| c.value().to_string());

    if let Some(sid) = session_id {
        if let Ok(Some(_user)) = auth.get_session(&sid).await {
            return next.run(req).await;
        }
    }

    (StatusCode::FOUND, [(LOCATION, "/admin/login")]).into_response()
}
