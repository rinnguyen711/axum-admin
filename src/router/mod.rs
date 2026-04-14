mod csrf;
mod helpers;
mod auth;
mod entity;
mod users;

use crate::{app::AdminApp, middleware::require_auth};
use axum::{
    extract::Path,
    http::header,
    middleware,
    response::{IntoResponse, Redirect},
    routing::{delete, get, post},
    Router,
};
use axum::extract::DefaultBodyLimit;
use tower_cookies::CookieManagerLayer;
use axum::extract::Extension;

async fn serve_htmx() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/javascript")],
        include_str!("../../static/htmx.min.js"),
    )
}

async fn serve_alpine() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/javascript")],
        include_str!("../../static/alpine.min.js"),
    )
}

async fn serve_admin_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css")],
        include_str!("../../static/admin.css"),
    )
}

impl AdminApp {
    pub async fn into_router(self) -> Router {
        let entity_names: Vec<String> = self
            .entities
            .iter()
            .map(|e| e.entity_name.clone())
            .collect();

        #[cfg(feature = "seaorm")]
        if let Some(ref seaorm_auth) = self.seaorm_auth {
            seaorm_auth
                .seed_roles(&entity_names)
                .await
                .expect("failed to seed RBAC roles");
        }

        let (auth, state, upload_limit) = self.into_state();

        let protected = Router::new()
            .route("/admin", get(|| async { Redirect::permanent("/admin/") }))
            .route("/admin/", get(entity::admin_home))
            .route("/admin/logout", get(auth::logout))
            .route("/admin/change-password", get(auth::change_password_page))
            .route("/admin/change-password", post(auth::change_password_submit))
            .route("/admin/users/", get(users::user_list))
            .route("/admin/users/new", get(users::user_create_form))
            .route("/admin/users/new", post(users::user_create_submit))
            .route("/admin/users/:id/", get(users::user_edit_form))
            .route("/admin/users/:id/", post(users::user_edit_submit))
            .route("/admin/users/:id/delete", delete(users::user_delete))
            .route("/admin/:entity", get(|Path(e): Path<String>| async move {
                Redirect::permanent(&format!("/admin/{}/", e))
            }))
            .route("/admin/:entity/", get(entity::entity_list))
            .route("/admin/:entity/new", get(entity::entity_create_form))
            .route("/admin/:entity/new", post(entity::entity_create_submit))
            .route("/admin/:entity/:id/", get(entity::entity_edit_form))
            .route("/admin/:entity/:id/", post(entity::entity_edit_submit))
            .route("/admin/:entity/:id/delete", delete(entity::entity_delete))
            .route("/admin/:entity/action/:action_name", post(entity::entity_action))
            .route_layer(middleware::from_fn(require_auth))
            .layer(DefaultBodyLimit::max(upload_limit));

        Router::new()
            .route("/admin/login", get(auth::login_page))
            .route("/admin/login", post(auth::login_submit))
            .route("/admin/_static/htmx.min.js", get(serve_htmx))
            .route("/admin/_static/alpine.min.js", get(serve_alpine))
            .route("/admin/_static/admin.css", get(serve_admin_css))
            .merge(protected)
            .layer(Extension(state))
            .layer(Extension(auth))
            .layer(CookieManagerLayer::new())
    }
}
