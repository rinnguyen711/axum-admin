use axum_admin::auth::{AdminAuth, DefaultAdminAuth};
use axum_admin::AdminError;

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
