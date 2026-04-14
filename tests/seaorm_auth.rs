#[cfg(feature = "seaorm")]
mod tests {
    use sea_orm::{Database, DbErr};

    async fn setup_db() -> Result<sea_orm::DatabaseConnection, DbErr> {
        let db = Database::connect("sqlite::memory:").await?;
        use sea_orm::ConnectionTrait;
        db.execute_unprepared(
            "CREATE TABLE auth_users (
                id TEXT PRIMARY KEY,
                username TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                is_superuser INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )"
        ).await?;
        db.execute_unprepared(
            "CREATE TABLE casbin_rule (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ptype TEXT NOT NULL DEFAULT '',
                v0 TEXT NOT NULL DEFAULT '',
                v1 TEXT NOT NULL DEFAULT '',
                v2 TEXT NOT NULL DEFAULT '',
                v3 TEXT NOT NULL DEFAULT '',
                v4 TEXT NOT NULL DEFAULT '',
                v5 TEXT NOT NULL DEFAULT ''
            )"
        ).await?;
        Ok(db)
    }

    async fn setup_db_with_casbin() -> sea_orm::DatabaseConnection {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        use sea_orm::ConnectionTrait;
        db.execute_unprepared(
            "CREATE TABLE auth_users (
                id TEXT PRIMARY KEY,
                username TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                is_superuser INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )"
        ).await.unwrap();
        db.execute_unprepared(
            "CREATE TABLE casbin_rule (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ptype TEXT NOT NULL DEFAULT '',
                v0 TEXT NOT NULL DEFAULT '',
                v1 TEXT NOT NULL DEFAULT '',
                v2 TEXT NOT NULL DEFAULT '',
                v3 TEXT NOT NULL DEFAULT '',
                v4 TEXT NOT NULL DEFAULT '',
                v5 TEXT NOT NULL DEFAULT ''
            )"
        ).await.unwrap();
        db
    }

    #[tokio::test]
    async fn casbin_superuser_bypasses_enforcer() {
        use axum_admin::adapters::seaorm_auth::SeaOrmAdminAuth;
        use axum_admin::auth::{check_permission, AdminAuth};

        let db = setup_db_with_casbin().await;
        let auth = SeaOrmAdminAuth::new(db).await.unwrap();
        auth.ensure_user("admin", "secret").await.unwrap();

        let user = auth.authenticate("admin", "secret").await.unwrap();
        assert!(user.is_superuser);
        // superuser passes any check with no enforcer
        assert!(check_permission(&user, &Some("posts.delete".into()), None));
    }

    #[tokio::test]
    async fn casbin_enforcer_grants_permission() {
        use axum_admin::adapters::seaorm_auth::SeaOrmAdminAuth;
        use axum_admin::auth::{check_permission, AdminUser};
        use casbin::MgmtApi;

        let db = setup_db_with_casbin().await;
        let auth = SeaOrmAdminAuth::new(db).await.unwrap();

        // add policy: alice can view posts
        {
            let arc = auth.enforcer();
            let mut enforcer = arc.write().await;
            enforcer.add_policy(vec![
                "alice".to_string(), "posts".to_string(), "view".to_string()
            ]).await.unwrap();
        }

        let user = AdminUser { username: "alice".into(), session_id: "s".into(), is_superuser: false };
        let enforcer = auth.enforcer();
        assert!(check_permission(&user, &Some("posts.view".into()), Some(&enforcer)));
        assert!(!check_permission(&user, &Some("posts.delete".into()), Some(&enforcer)));
    }

    #[tokio::test]
    async fn create_and_find_user() {
        let db = setup_db().await.unwrap();
        use axum_admin::adapters::seaorm_auth::{AuthUserActiveModel, AuthUserEntity};
        use sea_orm::{ActiveModelTrait, EntityTrait, Set};

        let user = AuthUserActiveModel {
            id: Set("id-1".to_string()),
            username: Set("alice".to_string()),
            password_hash: Set("hash".to_string()),
            is_active: Set(true),
            is_superuser: Set(false),
            created_at: Set(chrono::Utc::now().naive_utc()),
            updated_at: Set(chrono::Utc::now().naive_utc()),
        };
        user.insert(&db).await.unwrap();

        let found = AuthUserEntity::find_by_id("id-1".to_string())
            .one(&db)
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().username, "alice");
    }

    #[tokio::test]
    async fn seaorm_auth_authenticate_correct() {
        use axum_admin::adapters::seaorm_auth::SeaOrmAdminAuth;
        use axum_admin::auth::AdminAuth;

        let db = setup_db().await.unwrap();
        let auth = SeaOrmAdminAuth::new(db).await.unwrap();
        auth.ensure_user("admin", "secret").await.unwrap();

        let user = auth.authenticate("admin", "secret").await.unwrap();
        assert_eq!(user.username, "admin");
        assert!(!user.session_id.is_empty());
    }

    #[tokio::test]
    async fn seaorm_auth_authenticate_wrong_password() {
        use axum_admin::adapters::seaorm_auth::SeaOrmAdminAuth;
        use axum_admin::auth::AdminAuth;
        use axum_admin::AdminError;

        let db = setup_db().await.unwrap();
        let auth = SeaOrmAdminAuth::new(db).await.unwrap();
        auth.ensure_user("admin", "secret").await.unwrap();

        let result = auth.authenticate("admin", "wrong").await;
        assert!(matches!(result, Err(AdminError::Unauthorized)));
    }

    #[tokio::test]
    async fn seaorm_auth_session_roundtrip() {
        use axum_admin::adapters::seaorm_auth::SeaOrmAdminAuth;
        use axum_admin::auth::AdminAuth;

        let db = setup_db().await.unwrap();
        let auth = SeaOrmAdminAuth::new(db).await.unwrap();
        auth.ensure_user("admin", "secret").await.unwrap();

        let user = auth.authenticate("admin", "secret").await.unwrap();
        let found = auth.get_session(&user.session_id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().username, "admin");
    }

    #[tokio::test]
    async fn seaorm_auth_ensure_user_idempotent() {
        use axum_admin::adapters::seaorm_auth::SeaOrmAdminAuth;

        let db = setup_db().await.unwrap();
        let auth = SeaOrmAdminAuth::new(db).await.unwrap();
        auth.ensure_user("admin", "secret").await.unwrap();
        // second call should not fail or create a duplicate
        auth.ensure_user("admin", "secret").await.unwrap();

        use axum_admin::auth::AdminAuth;
        let user = auth.authenticate("admin", "secret").await.unwrap();
        assert_eq!(user.username, "admin");
    }

    #[tokio::test]
    async fn users_list_requires_auth() {
        use axum_admin::adapters::seaorm_auth::SeaOrmAdminAuth;
        use axum_test::TestServer;
        use axum::http::StatusCode;

        let db = setup_db_with_casbin().await;
        let auth = SeaOrmAdminAuth::new(db).await.unwrap();
        auth.ensure_user("admin", "secret").await.unwrap();

        let app = axum_admin::AdminApp::new()
            .seaorm_auth(auth)
            .into_router()
            .await;

        let server = TestServer::new(app).unwrap();
        // Unauthenticated request should redirect to login (302)
        let resp = server.get("/admin/users/").await;
        assert_eq!(resp.status_code(), StatusCode::FOUND);
    }

    #[tokio::test]
    async fn change_password_page_returns_200() {
        use axum_admin::adapters::seaorm_auth::SeaOrmAdminAuth;
        use axum_test::{TestServer, TestServerConfig};
        use axum::http::StatusCode;

        let db = setup_db_with_casbin().await;
        let auth = SeaOrmAdminAuth::new(db).await.unwrap();
        auth.ensure_user("admin", "secret").await.unwrap();

        let app = axum_admin::AdminApp::new()
            .seaorm_auth(auth)
            .into_router()
            .await;

        let config = TestServerConfig {
            save_cookies: true,
            ..TestServerConfig::default()
        };
        let server = TestServer::new_with_config(app, config).unwrap();

        // Login to get session cookie
        let login = server
            .post("/admin/login")
            .form(&[("username", "admin"), ("password", "secret")])
            .await;
        assert_eq!(login.status_code(), StatusCode::FOUND);

        // Change password page returns 200
        let resp = server.get("/admin/change-password").await;
        assert_eq!(resp.status_code(), StatusCode::OK);
    }
}
