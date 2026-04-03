use axum::http::StatusCode;
use axum_admin::auth::{AdminAuth, DefaultAdminAuth};
use axum_admin::{AdminApp, AdminError};
use axum_test::TestServer;

#[tokio::test]
async fn default_auth_correct_password() {
    let auth = DefaultAdminAuth::new().add_user("admin", "secret123");
    let user = auth.authenticate("admin", "secret123").await.unwrap();
    assert_eq!(user.username, "admin");
    assert!(!user.session_id.is_empty());
}

#[tokio::test]
async fn default_auth_wrong_password() {
    let auth = DefaultAdminAuth::new().add_user("admin", "secret123");
    let result = auth.authenticate("admin", "wrongpass").await;
    assert!(matches!(result, Err(AdminError::Unauthorized)));
}

#[tokio::test]
async fn default_auth_unknown_user() {
    let auth = DefaultAdminAuth::new().add_user("admin", "secret123");
    let result = auth.authenticate("nobody", "secret123").await;
    assert!(matches!(result, Err(AdminError::Unauthorized)));
}

#[tokio::test]
async fn default_auth_session_roundtrip() {
    let auth = DefaultAdminAuth::new().add_user("admin", "secret123");
    let user = auth.authenticate("admin", "secret123").await.unwrap();
    let session_id = user.session_id.clone();

    let found = auth.get_session(&session_id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().username, "admin");
}

#[tokio::test]
async fn default_auth_invalid_session() {
    let auth = DefaultAdminAuth::new().add_user("admin", "secret123");
    let found = auth.get_session("nonexistent-session-id").await.unwrap();
    assert!(found.is_none());
}

#[test]
fn admin_app_with_auth() {
    let app = AdminApp::new()
        .title("Test Admin")
        .auth(Box::new(DefaultAdminAuth::new().add_user("admin", "pass")));

    assert!(app.auth.is_some());
    assert_eq!(app.title, "Test Admin");
}

#[tokio::test]
async fn login_page_returns_200() {
    let app = AdminApp::new()
        .auth(Box::new(DefaultAdminAuth::new().add_user("admin", "secret")))
        .into_router();

    let server = TestServer::new(app).unwrap();
    let resp = server.get("/admin/login").await;
    assert_eq!(resp.status_code(), StatusCode::OK);
}

#[tokio::test]
async fn admin_root_redirects_to_login_when_unauthenticated() {
    let app = AdminApp::new()
        .auth(Box::new(DefaultAdminAuth::new().add_user("admin", "secret")))
        .into_router();

    let server = TestServer::new(app).unwrap();
    let resp = server.get("/admin/").await;
    assert_eq!(resp.status_code(), StatusCode::FOUND);
    assert!(resp.headers().get("location").unwrap().to_str().unwrap().contains("/admin/login"));
}

#[tokio::test]
async fn login_post_correct_credentials_redirects_to_admin() {
    let app = AdminApp::new()
        .auth(Box::new(DefaultAdminAuth::new().add_user("admin", "secret")))
        .into_router();

    let server = TestServer::new(app).unwrap();
    let resp = server
        .post("/admin/login")
        .form(&[("username", "admin"), ("password", "secret")])
        .await;
    assert_eq!(resp.status_code(), StatusCode::FOUND);
    assert!(resp.headers().get("location").unwrap().to_str().unwrap().contains("/admin/"));
}

#[tokio::test]
async fn login_post_wrong_credentials_returns_login_page() {
    let app = AdminApp::new()
        .auth(Box::new(DefaultAdminAuth::new().add_user("admin", "secret")))
        .into_router();

    let server = TestServer::new(app).unwrap();
    let resp = server
        .post("/admin/login")
        .form(&[("username", "admin"), ("password", "wrong")])
        .await;
    assert_eq!(resp.status_code(), StatusCode::OK);
}

#[test]
fn admin_user_superuser_constructor() {
    let user = axum_admin::auth::AdminUser::superuser("alice", "sess-1");
    assert_eq!(user.username, "alice");
    assert_eq!(user.session_id, "sess-1");
    assert!(user.permissions.is_empty());
}

#[test]
fn has_permission_none_required_always_true() {
    let user = axum_admin::auth::AdminUser { username: "a".into(), session_id: "s".into(), permissions: vec!["x".into()] };
    assert!(axum_admin::auth::has_permission(&user, &None));
}

#[test]
fn has_permission_empty_permissions_is_superuser() {
    let user = axum_admin::auth::AdminUser { username: "a".into(), session_id: "s".into(), permissions: vec![] };
    assert!(axum_admin::auth::has_permission(&user, &Some("anything".into())));
}

#[test]
fn has_permission_grants_when_present() {
    let user = axum_admin::auth::AdminUser { username: "a".into(), session_id: "s".into(), permissions: vec!["posts.view".into()] };
    assert!(axum_admin::auth::has_permission(&user, &Some("posts.view".into())));
}

#[test]
fn has_permission_denies_when_absent() {
    let user = axum_admin::auth::AdminUser { username: "a".into(), session_id: "s".into(), permissions: vec!["posts.view".into()] };
    assert!(!axum_admin::auth::has_permission(&user, &Some("posts.delete".into())));
}
