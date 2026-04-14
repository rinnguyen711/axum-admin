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
        .into_router()
        .await;

    let server = TestServer::new(app).unwrap();
    let resp = server.get("/admin/login").await;
    assert_eq!(resp.status_code(), StatusCode::OK);
}

#[tokio::test]
async fn admin_root_redirects_to_login_when_unauthenticated() {
    let app = AdminApp::new()
        .auth(Box::new(DefaultAdminAuth::new().add_user("admin", "secret")))
        .into_router()
        .await;

    let server = TestServer::new(app).unwrap();
    let resp = server.get("/admin/").await;
    assert_eq!(resp.status_code(), StatusCode::FOUND);
    assert!(resp.headers().get("location").unwrap().to_str().unwrap().contains("/admin/login"));
}

#[tokio::test]
async fn login_post_correct_credentials_redirects_to_admin() {
    let app = AdminApp::new()
        .auth(Box::new(DefaultAdminAuth::new().add_user("admin", "secret")))
        .into_router()
        .await;

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
        .into_router()
        .await;

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
    assert!(user.is_superuser);
}

#[test]
fn admin_user_has_is_superuser_field() {
    let user = axum_admin::auth::AdminUser {
        username: "alice".into(),
        session_id: "s".into(),
        is_superuser: true,
    };
    assert!(user.is_superuser);
}

#[cfg(feature = "seaorm")]
mod seaorm_auth_tests {
    use axum_admin::adapters::migrations::Migrator;
    use sea_orm::{Database, DatabaseConnection};
    use sea_orm_migration::MigratorTrait;

    async fn in_memory_db() -> DatabaseConnection {
        Database::connect("sqlite::memory:")
            .await
            .expect("failed to connect")
    }

    #[tokio::test]
    async fn migrations_run_and_are_idempotent() {
        let db = in_memory_db().await;

        // First run — should create tables
        Migrator::up(&db, None).await.expect("first migration failed");

        // Second run — should be a no-op (IF NOT EXISTS)
        Migrator::up(&db, None).await.expect("second migration failed");
    }

    #[tokio::test]
    async fn seaorm_admin_auth_new_runs_migrations() {
        use axum_admin::SeaOrmAdminAuth;
        let db = in_memory_db().await;
        // SeaOrmAdminAuth::new() should succeed, which means migrations ran
        SeaOrmAdminAuth::new(db).await.expect("SeaOrmAdminAuth::new failed");
    }
}
