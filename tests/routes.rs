use axum::http::StatusCode;
use axum_test::{TestServer, TestServerConfig};
use axum_admin::{AdminApp, EntityAdmin, Field};
use axum_admin::auth::DefaultAdminAuth;
use std::collections::HashMap;
use serde_json::Value;
use async_trait::async_trait;
use axum_admin::{DataAdapter, ListParams, AdminError};

struct StubAdapter;

#[async_trait]
impl DataAdapter for StubAdapter {
    async fn list(&self, _p: ListParams) -> Result<Vec<HashMap<String, Value>>, AdminError> {
        Ok(vec![
            HashMap::from([
                ("id".to_string(), Value::from(1)),
                ("name".to_string(), Value::from("Alice")),
            ]),
        ])
    }
    async fn get(&self, _id: &Value) -> Result<HashMap<String, Value>, AdminError> {
        Ok(HashMap::from([
            ("id".to_string(), Value::from(1)),
            ("name".to_string(), Value::from("Alice")),
        ]))
    }
    async fn create(&self, _d: HashMap<String, Value>) -> Result<Value, AdminError> { Ok(Value::from(2)) }
    async fn update(&self, _id: &Value, _d: HashMap<String, Value>) -> Result<(), AdminError> { Ok(()) }
    async fn delete(&self, _id: &Value) -> Result<(), AdminError> { Ok(()) }
    async fn count(&self, _p: &ListParams) -> Result<u64, AdminError> { Ok(1) }
}

async fn make_app() -> axum::Router {
    AdminApp::new()
        .auth(Box::new(DefaultAdminAuth::new().add_user("admin", "secret")))
        .register(
            EntityAdmin::new::<()>("users")
                .label("Users")
                .field(Field::number("id").readonly())
                .field(Field::text("name").required())
                .list_display(vec!["id".to_string(), "name".to_string()])
                .adapter(Box::new(StubAdapter)),
        )
        .into_router()
        .await
}

async fn make_server() -> TestServer {
    let config = TestServerConfig {
        save_cookies: true,
        ..TestServerConfig::default()
    };
    TestServer::new_with_config(make_app().await, config).unwrap()
}

#[tokio::test]
async fn list_page_renders_entity_rows() {
    let server = make_server().await;

    // Log in first
    let login = server
        .post("/admin/login")
        .form(&[("username", "admin"), ("password", "secret")])
        .await;
    assert_eq!(login.status_code(), StatusCode::FOUND);

    // Follow session cookie automatically with axum-test
    let resp = server.get("/admin/users/").await;
    assert_eq!(resp.status_code(), StatusCode::OK);
    let body = resp.text();
    assert!(body.contains("Alice"), "Expected 'Alice' in list page");
    assert!(body.contains("Users"), "Expected entity label in page");
}

#[tokio::test]
async fn create_form_renders() {
    let server = make_server().await;
    server.post("/admin/login").form(&[("username", "admin"), ("password", "secret")]).await;

    let resp = server.get("/admin/users/new").await;
    assert_eq!(resp.status_code(), StatusCode::OK);
    let body = resp.text();
    assert!(body.contains("name"), "Expected field 'name' in form");
}

#[tokio::test]
async fn edit_form_renders() {
    let server = make_server().await;
    server.post("/admin/login").form(&[("username", "admin"), ("password", "secret")]).await;

    let resp = server.get("/admin/users/1/").await;
    assert_eq!(resp.status_code(), StatusCode::OK);
    let body = resp.text();
    assert!(body.contains("Alice"), "Expected record value in edit form");
}

#[tokio::test]
async fn delete_redirects() {
    let server = make_server().await;
    server.post("/admin/login").form(&[("username", "admin"), ("password", "secret")]).await;

    let resp = server.delete("/admin/users/1/delete").await;
    assert_eq!(resp.status_code(), StatusCode::FOUND);
}

struct FkStubAdapter;

#[async_trait]
impl DataAdapter for FkStubAdapter {
    async fn list(&self, _p: ListParams) -> Result<Vec<HashMap<String, Value>>, AdminError> {
        Ok(vec![
            HashMap::from([("id".to_string(), Value::from(1)), ("name".to_string(), Value::from("Tech"))]),
            HashMap::from([("id".to_string(), Value::from(2)), ("name".to_string(), Value::from("Rust"))]),
        ])
    }
    async fn get(&self, _id: &Value) -> Result<HashMap<String, Value>, AdminError> {
        Ok(HashMap::from([("id".to_string(), Value::from(1)), ("category_id".to_string(), Value::from(1))]))
    }
    async fn create(&self, _d: HashMap<String, Value>) -> Result<Value, AdminError> { Ok(Value::from(1)) }
    async fn update(&self, _id: &Value, _d: HashMap<String, Value>) -> Result<(), AdminError> { Ok(()) }
    async fn delete(&self, _id: &Value) -> Result<(), AdminError> { Ok(()) }
    async fn count(&self, _p: &ListParams) -> Result<u64, AdminError> { Ok(1) }
}

async fn make_fk_app() -> axum::Router {
    AdminApp::new()
        .auth(Box::new(DefaultAdminAuth::new().add_user("admin", "secret")))
        .register(
            EntityAdmin::new::<()>("posts")
                .label("Posts")
                .field(Field::text("title").required())
                .field(Field::foreign_key(
                    "category_id",
                    "Category",
                    Box::new(FkStubAdapter),
                    "id",
                    "name",
                ))
                .list_display(vec!["id".to_string(), "title".to_string()])
                .adapter(Box::new(FkStubAdapter)),
        )
        .into_router()
        .await
}

#[tokio::test]
async fn fk_field_renders_select_with_options() {
    let config = TestServerConfig { save_cookies: true, ..TestServerConfig::default() };
    let server = TestServer::new_with_config(make_fk_app().await, config).unwrap();
    server.post("/admin/login").form(&[("username", "admin"), ("password", "secret")]).await;

    let resp = server.get("/admin/posts/1/").await;
    assert_eq!(resp.status_code(), StatusCode::OK);
    let body = resp.text();
    assert!(body.contains("Tech"), "Expected FK option 'Tech' in form");
    assert!(body.contains("Rust"), "Expected FK option 'Rust' in form");
    assert!(body.contains(r#"name="category_id""#), "Expected category_id field");
    assert!(body.contains("<option"), "Expected <option> elements for FK field");
}

#[tokio::test]
async fn file_upload_create_stores_file() {
    use std::sync::Arc;
    use axum_admin::{LocalStorage, Field};

    struct FileStubAdapter;

    #[async_trait]
    impl DataAdapter for FileStubAdapter {
        async fn list(&self, _p: ListParams) -> Result<Vec<HashMap<String, Value>>, AdminError> { Ok(vec![]) }
        async fn get(&self, _id: &Value) -> Result<HashMap<String, Value>, AdminError> {
            Ok(HashMap::from([
                ("id".to_string(), Value::from(1)),
                ("avatar".to_string(), Value::Null),
            ]))
        }
        async fn create(&self, data: HashMap<String, Value>) -> Result<Value, AdminError> {
            if let Some(Value::String(url)) = data.get("avatar") {
                assert!(url.starts_with("/media/"), "url should start with /media/, got: {url}");
            } else {
                panic!("avatar should be a String URL in data, got: {:?}", data.get("avatar"));
            }
            Ok(Value::from(1))
        }
        async fn update(&self, _id: &Value, _d: HashMap<String, Value>) -> Result<(), AdminError> { Ok(()) }
        async fn delete(&self, _id: &Value) -> Result<(), AdminError> { Ok(()) }
        async fn count(&self, _p: &ListParams) -> Result<u64, AdminError> { Ok(0) }
    }

    let dir = tempfile::tempdir().unwrap();
    let storage = Arc::new(LocalStorage::new(dir.path(), "/media"));

    let app = AdminApp::new()
        .auth(Box::new(axum_admin::auth::DefaultAdminAuth::new().add_user("admin", "secret")))
        .register(
            EntityAdmin::new::<()>("items")
                .field(Field::image("avatar", storage))
                .adapter(Box::new(FileStubAdapter)),
        )
        .into_router()
        .await;

    let config = axum_test::TestServerConfig { save_cookies: true, ..Default::default() };
    let server = axum_test::TestServer::new_with_config(app, config).unwrap();

    // Log in
    let login = server.post("/admin/login")
        .form(&[("username", "admin"), ("password", "secret")])
        .await;
    assert_eq!(login.status_code(), 302);

    // Get CSRF token from create form
    let form_page = server.get("/admin/items/new").await;
    let body = form_page.text();
    let csrf_start = body.find("name=\"csrf_token\" value=\"").unwrap() + 25;
    let csrf_end = body[csrf_start..].find('"').unwrap() + csrf_start;
    let csrf_token = body[csrf_start..csrf_end].to_string();

    // Submit multipart form with a fake image file
    let response = server
        .post("/admin/items/new")
        .multipart(
            axum_test::multipart::MultipartForm::new()
                .add_text("csrf_token", &csrf_token)
                .add_part(
                    "avatar",
                    axum_test::multipart::Part::bytes(b"fakepng".to_vec())
                        .file_name("test.png")
                        .mime_type("image/png"),
                ),
        )
        .await;

    // Should redirect to list on success
    assert_eq!(response.status_code(), 303, "expected redirect, got: {}", response.text());

    // File should exist on disk
    let files: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
    assert_eq!(files.len(), 1, "expected 1 file in upload dir");
}
