use axum_admin::AdminError;
use axum_admin::{Field, FieldType};
use axum_admin::{DataAdapter, ListParams};
use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::Value;

#[test]
fn admin_error_display() {
    let e = AdminError::NotFound;
    assert_eq!(e.to_string(), "not found");

    let mut fields = HashMap::new();
    fields.insert("email".to_string(), "is required".to_string());
    let e = AdminError::ValidationError(fields);
    assert!(e.to_string().contains("validation error"));

    let e = AdminError::DatabaseError("connection refused".to_string());
    assert!(e.to_string().contains("connection refused"));

    let e = AdminError::Unauthorized;
    assert_eq!(e.to_string(), "unauthorized");

    let e = AdminError::Custom("something went wrong".to_string());
    assert!(e.to_string().contains("something went wrong"));
}

#[test]
fn field_builder_text() {
    let f = Field::text("name");
    assert_eq!(f.name, "name");
    assert_eq!(f.label, "Name"); // auto-capitalised from field name
    assert!(!f.readonly);
    assert!(!f.hidden);
    assert!(!f.required);
    assert!(matches!(f.field_type, FieldType::Text));
}

#[test]
fn field_builder_chainable() {
    let f = Field::email("email")
        .label("Email Address")
        .required()
        .help_text("Must be unique");
    assert_eq!(f.label, "Email Address");
    assert!(f.required);
    assert_eq!(f.help_text, Some("Must be unique".to_string()));
    assert!(matches!(f.field_type, FieldType::Email));
}

#[test]
fn field_builder_select() {
    let f = Field::select(
        "status",
        vec![("active".to_string(), "Active".to_string()), ("banned".to_string(), "Banned".to_string())],
    );
    assert!(matches!(f.field_type, FieldType::Select(_)));
}

#[test]
fn field_foreign_key_constructor() {
    let adapter = Box::new(MockAdapter);
    let f = Field::foreign_key("category_id", "Category", adapter, "id", "name");
    assert_eq!(f.name, "category_id");
    assert_eq!(f.label, "Category");
    assert!(matches!(f.field_type, FieldType::ForeignKey { .. }));
    assert!(!f.required);
}

#[test]
fn field_foreign_key_modifiers() {
    let f = Field::foreign_key("cat_id", "Cat", Box::new(MockAdapter), "id", "name")
        .fk_limit(50)
        .fk_order_by("name");
    if let FieldType::ForeignKey { limit, order_by, .. } = f.field_type {
        assert_eq!(limit, Some(50));
        assert_eq!(order_by, Some("name".to_string()));
    } else {
        panic!("expected ForeignKey variant");
    }
}

#[test]
fn field_modifiers() {
    let f = Field::number("age").readonly();
    assert!(f.readonly);

    let f = Field::text("secret").hidden();
    assert!(f.hidden);

    let f = Field::text("note").list_only();
    assert!(f.list_only);

    let f = Field::text("bio").form_only();
    assert!(f.form_only);
}

#[test]
fn field_upsert_replaces_by_name() {
    struct User;
    let entity = EntityAdmin::new::<User>("users")
        .field(Field::text("name"))
        .field(Field::text("name").required()); // second call with same name should replace

    assert_eq!(entity.fields.len(), 1, "should have exactly one 'name' field");
    assert!(entity.fields[0].required, "replaced field should be required");
}

#[test]
fn field_upsert_appends_when_new_name() {
    struct User;
    let entity = EntityAdmin::new::<User>("users")
        .field(Field::text("name"))
        .field(Field::text("email"));

    assert_eq!(entity.fields.len(), 2);
    assert_eq!(entity.fields[0].name, "name");
    assert_eq!(entity.fields[1].name, "email");
}

struct MockAdapter;

#[async_trait]
impl DataAdapter for MockAdapter {
    async fn list(&self, _params: ListParams) -> Result<Vec<HashMap<String, Value>>, AdminError> {
        Ok(vec![
            HashMap::from([("id".to_string(), Value::from(1)), ("name".to_string(), Value::from("Alice"))]),
        ])
    }
    async fn get(&self, _id: &Value) -> Result<HashMap<String, Value>, AdminError> {
        Ok(HashMap::from([("id".to_string(), Value::from(1))]))
    }
    async fn create(&self, _data: HashMap<String, Value>) -> Result<Value, AdminError> {
        Ok(Value::from(42))
    }
    async fn update(&self, _id: &Value, _data: HashMap<String, Value>) -> Result<(), AdminError> {
        Ok(())
    }
    async fn delete(&self, _id: &Value) -> Result<(), AdminError> {
        Ok(())
    }
    async fn count(&self, _params: &ListParams) -> Result<u64, AdminError> {
        Ok(1)
    }
}

#[tokio::test]
async fn data_adapter_mock() {
    let adapter = MockAdapter;
    let params = ListParams {
        page: 1,
        per_page: 20,
        search: None,
        search_columns: Vec::new(),
        filters: HashMap::new(),
        order_by: None,
    };
    let rows = adapter.list(params).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["name"], Value::from("Alice"));

    let count = adapter.count(&ListParams {
        page: 1,
        per_page: 20,
        search: None,
        search_columns: Vec::new(),
        filters: HashMap::new(),
        order_by: None,
    }).await.unwrap();
    assert_eq!(count, 1);
}

#[test]
fn list_params_defaults() {
    let p = ListParams::default();
    assert_eq!(p.page, 1);
    assert_eq!(p.per_page, 20);
    assert!(p.search.is_none());
    assert!(p.order_by.is_none());
}

use axum_admin::{EntityAdmin, ActionTarget, ActionContext, ActionResult};
use axum_admin::entity::CustomAction;

struct User;

#[tokio::test]
async fn entity_admin_builder_basic() {
    let entity = EntityAdmin::new::<User>("users")
        .label("Users")
        .field(Field::text("name"))
        .field(Field::email("email").required())
        .list_display(vec!["name".to_string(), "email".to_string()])
        .search_fields(vec!["name".to_string(), "email".to_string()])
        .adapter(Box::new(MockAdapter));

    assert_eq!(entity.label, "Users");
    assert_eq!(entity.entity_name, "users");
    assert_eq!(entity.fields.len(), 2);
    assert_eq!(entity.list_display, vec!["name", "email"]);
    assert_eq!(entity.search_fields, vec!["name", "email"]);
    assert!(entity.adapter.is_some());
}

#[tokio::test]
async fn entity_admin_custom_action() {
    let entity = EntityAdmin::new::<User>("users")
        .adapter(Box::new(MockAdapter))
        .action(
            CustomAction::builder("ban", "Ban Users")
                .target(ActionTarget::List)
                .confirm("Sure?")
                .handler(|_ctx| Box::pin(async { Ok(ActionResult::Success("Banned".to_string())) })),
        );

    assert_eq!(entity.actions.len(), 1);
    assert_eq!(entity.actions[0].name, "ban");
    assert_eq!(entity.actions[0].label, "Ban Users");
    assert!(entity.actions[0].confirm.is_some());
    assert!(matches!(entity.actions[0].target, ActionTarget::List));

    // invoke the handler
    let ctx = ActionContext { ids: vec![Value::from(1)], params: HashMap::new() };
    let result = (entity.actions[0].handler)(ctx).await.unwrap();
    assert!(matches!(result, ActionResult::Success(_)));
}

#[test]
fn entity_admin_before_save_hook() {
    let mut data = HashMap::from([("name".to_string(), Value::from("  alice  "))]);
    let entity = EntityAdmin::new::<User>("users")
        .adapter(Box::new(MockAdapter))
        .before_save(|d| {
            if let Some(Value::String(s)) = d.get_mut("name") {
                *s = s.trim().to_string();
            }
            Ok(())
        });

    if let Some(hook) = &entity.before_save {
        hook(&mut data).unwrap();
    }
    assert_eq!(data["name"], Value::from("alice"));
}

#[test]
fn entity_admin_filter_fields_sets_names() {
    struct User;
    let entity = EntityAdmin::new::<User>("users")
        .filter_fields(vec!["name", "email"]);
    assert_eq!(entity.filter_fields, vec!["name", "email"]);
}

#[test]
fn entity_admin_filter_upserts_by_name() {
    struct User;
    let entity = EntityAdmin::new::<User>("users")
        .filter(Field::text("status"))
        .filter(Field::text("status").required()); // second call replaces
    assert_eq!(entity.filters.len(), 1);
    assert!(entity.filters[0].required);
}

#[test]
fn entity_admin_filter_appends_new_name() {
    struct User;
    let entity = EntityAdmin::new::<User>("users")
        .filter(Field::text("status"))
        .filter(Field::text("category_id"));
    assert_eq!(entity.filters.len(), 2);
    assert_eq!(entity.filters[0].name, "status");
    assert_eq!(entity.filters[1].name, "category_id");
}

use axum_admin::AdminApp;

#[test]
fn admin_app_builder() {
    let app = AdminApp::new()
        .title("My Admin")
        .prefix("/admin")
        .register(
            EntityAdmin::new::<User>("users")
                .label("Users")
                .adapter(Box::new(MockAdapter)),
        )
        .register(
            EntityAdmin::new::<User>("posts")
                .label("Posts")
                .adapter(Box::new(MockAdapter)),
        );

    assert_eq!(app.title, "My Admin");
    assert_eq!(app.prefix, "/admin");
    assert_eq!(app.entities.len(), 2);
    assert_eq!(app.entities[0].entity_name, "users");
    assert_eq!(app.entities[1].entity_name, "posts");
}
