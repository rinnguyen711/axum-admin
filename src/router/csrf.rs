use tower_cookies::{Cookie, Cookies};

pub(super) const CSRF_COOKIE: &str = "axum_admin_csrf";

fn generate_csrf_token() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Returns the current CSRF token from the cookie, creating one if absent.
pub(super) fn get_or_create_csrf(cookies: &Cookies) -> String {
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
pub(super) fn validate_csrf(cookies: &Cookies, form_token: Option<&str>) -> bool {
    match (cookies.get(CSRF_COOKIE), form_token) {
        (Some(cookie), Some(form)) => !form.is_empty() && cookie.value() == form,
        _ => false,
    }
}
