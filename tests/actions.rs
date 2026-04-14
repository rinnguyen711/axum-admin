use axum::http::StatusCode;
use axum_test::{TestServer, TestServerConfig};
use axum_admin::{AdminApp, EntityAdmin, AdminError};
use axum_admin::auth::DefaultAdminAuth;
use axum_admin::entity::{CustomAction, ActionTarget, ActionResult};
use axum_admin::{DataAdapter, ListParams};
use std::collections::HashMap;
use serde_json::Value;
use async_trait::async_trait;

struct StubAdapter;

#[async_trait]
impl DataAdapter for StubAdapter {
    async fn list(&self, _p: ListParams) -> Result<Vec<HashMap<String, Value>>, AdminError> { Ok(vec![]) }
    async fn get(&self, _id: &Value) -> Result<HashMap<String, Value>, AdminError> { Err(AdminError::NotFound) }
    async fn create(&self, _d: HashMap<String, Value>) -> Result<Value, AdminError> { Ok(Value::Null) }
    async fn update(&self, _id: &Value, _d: HashMap<String, Value>) -> Result<(), AdminError> { Ok(()) }
    async fn delete(&self, _id: &Value) -> Result<(), AdminError> { Ok(()) }
    async fn count(&self, _p: &ListParams) -> Result<u64, AdminError> { Ok(0) }
}

async fn make_app_with_action() -> axum::Router {
    AdminApp::new()
        .auth(Box::new(DefaultAdminAuth::new().add_user("admin", "secret")))
        .register(
            EntityAdmin::new::<()>("users")
                .label("Users")
                .adapter(Box::new(StubAdapter))
                .action(
                    CustomAction::builder("ban", "Ban Users")
                        .target(ActionTarget::List)
                        .confirm("Sure?")
                        .handler(|ctx| Box::pin(async move {
                            if ctx.ids.is_empty() {
                                return Ok(ActionResult::Error("No users selected".to_string()));
                            }
                            Ok(ActionResult::Success(format!("Banned {} users", ctx.ids.len())))
                        })),
                )
                .action(
                    CustomAction::builder("view_detail", "View Detail")
                        .target(ActionTarget::Detail)
                        .handler(|ctx| Box::pin(async move {
                            Ok(ActionResult::Redirect(format!("/admin/users/{}/",
                                ctx.ids.first().and_then(|v| v.as_str()).unwrap_or(""))))
                        })),
                ),
        )
        .into_router()
        .await
}

async fn make_server() -> TestServer {
    let config = TestServerConfig {
        save_cookies: true,
        ..TestServerConfig::default()
    };
    TestServer::new_with_config(make_app_with_action().await, config).unwrap()
}

#[tokio::test]
async fn list_action_returns_flash_success() {
    let server = make_server().await;
    server.post("/admin/login").form(&[("username", "admin"), ("password", "secret")]).await;

    let resp = server
        .post("/admin/users/action/ban")
        .form(&[("selected_ids", "1"), ("selected_ids", "2")])
        .await;
    assert_eq!(resp.status_code(), StatusCode::OK);
    let body = resp.text();
    assert!(body.contains("Banned 2 users"), "Expected success message in response, got: {body}");
    assert!(body.contains("flash-success"), "Expected flash-success class");
}

#[tokio::test]
async fn list_action_returns_flash_error() {
    let server = make_server().await;
    server.post("/admin/login").form(&[("username", "admin"), ("password", "secret")]).await;

    let resp = server
        .post("/admin/users/action/ban")
        .form(&[] as &[(&str, &str)])
        .await;
    assert_eq!(resp.status_code(), StatusCode::OK);
    let body = resp.text();
    assert!(body.contains("No users selected"));
    assert!(body.contains("flash-error"));
}

#[tokio::test]
async fn detail_action_returns_redirect() {
    let server = make_server().await;
    server.post("/admin/login").form(&[("username", "admin"), ("password", "secret")]).await;

    let resp = server
        .post("/admin/users/action/view_detail")
        .form(&[("id", "42")])
        .await;
    // ActionResult::Redirect → HTTP 302
    assert_eq!(resp.status_code(), StatusCode::FOUND);
    assert!(resp.headers().get("location").unwrap().to_str().unwrap().contains("/admin/users/42/"));
}

#[tokio::test]
async fn unknown_action_returns_404() {
    let server = make_server().await;
    server.post("/admin/login").form(&[("username", "admin"), ("password", "secret")]).await;

    let resp = server
        .post("/admin/users/action/nonexistent")
        .form(&[] as &[(&str, &str)])
        .await;
    assert_eq!(resp.status_code(), StatusCode::NOT_FOUND);
}
