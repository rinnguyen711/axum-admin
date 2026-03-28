# axum-admin

A modern admin dashboard framework for [Axum](https://github.com/tokio-rs/axum). Register your entities and get a full CRUD dashboard — search, filtering, pagination, custom actions, and built-in authentication — with zero frontend build step.

Inspired by Django Admin and Laravel Nova.

## Features

- **CRUD out of the box** — list, create, edit, delete for any entity
- **Server-side rendering** — MiniJinja templates, no JavaScript framework required
- **HTMX** — partial page swaps for search, pagination, and flash messages
- **Alpine.js + Pico CSS** — embedded, no CDN or build step
- **Built-in auth** — session-based login with bcrypt passwords; swap in your own auth backend
- **Custom actions** — bulk and single-record actions with HTMX flash responses
- **ORM-agnostic** — implement `DataAdapter` for any data source
- **SeaORM adapter** — first-party adapter behind the `seaorm` feature flag

## Quick start

```toml
[dependencies]
axum-admin = "0.1"
tokio = { version = "1", features = ["full"] }
axum = "0.7"
```

```rust
use axum_admin::{AdminApp, EntityAdmin, Field};
use axum_admin::auth::DefaultAdminAuth;

#[tokio::main]
async fn main() {
    let app = AdminApp::new()
        .title("My App")
        .auth(Box::new(
            DefaultAdminAuth::new()
                .add_user("admin", "secret"),
        ))
        .register(
            EntityAdmin::new::<()>("posts")
                .label("Posts")
                .field(Field::number("id").readonly())
                .field(Field::text("title").required())
                .field(Field::textarea("body"))
                .field(Field::boolean("published"))
                .list_display(vec!["id".into(), "title".into(), "published".into()])
                .adapter(Box::new(MyPostsAdapter)),
        )
        .into_router();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

Then open `http://localhost:3000/admin/login`.

## Implementing `DataAdapter`

`DataAdapter` is the bridge between the admin and your database. Implement it for any ORM or data source:

```rust
use axum_admin::{DataAdapter, ListParams, AdminError};
use async_trait::async_trait;
use std::collections::HashMap;
use serde_json::Value;

struct MyPostsAdapter;

#[async_trait]
impl DataAdapter for MyPostsAdapter {
    async fn list(&self, params: ListParams) -> Result<Vec<HashMap<String, Value>>, AdminError> {
        // Query your database, return rows as HashMap<column, value>
        todo!()
    }

    async fn get(&self, id: &Value) -> Result<HashMap<String, Value>, AdminError> {
        todo!()
    }

    async fn create(&self, data: HashMap<String, Value>) -> Result<Value, AdminError> {
        // Return the new record's ID
        todo!()
    }

    async fn update(&self, id: &Value, data: HashMap<String, Value>) -> Result<(), AdminError> {
        todo!()
    }

    async fn delete(&self, id: &Value) -> Result<(), AdminError> {
        todo!()
    }

    async fn count(&self, params: &ListParams) -> Result<u64, AdminError> {
        todo!()
    }
}
```

## SeaORM adapter

Enable the `seaorm` feature and use `SeaOrmAdapter<E>` directly:

```toml
[dependencies]
axum-admin = { version = "0.1", features = ["seaorm"] }
sea-orm = { version = "1", features = ["sqlx-postgres", "runtime-tokio-rustls"] }
```

```rust
use axum_admin::adapters::seaorm::SeaOrmAdapter;
use sea_orm::Database;

let db = Database::connect("postgres://localhost/myapp").await?;

EntityAdmin::new::<()>("users")
    .label("Users")
    .field(Field::number("id").readonly())
    .field(Field::text("name").required())
    .field(Field::email("email").required())
    .list_display(vec!["id".into(), "name".into(), "email".into()])
    .search_fields(vec!["name".into(), "email".into()])
    .adapter(Box::new(
        SeaOrmAdapter::<users::Entity>::new(db)
            .search_columns(vec!["name".into(), "email".into()]),
    ))
```

## Field types

| Builder method | HTML input | Notes |
|---|---|---|
| `Field::text("name")` | `<input type="text">` | |
| `Field::textarea("body")` | `<textarea>` | |
| `Field::email("email")` | `<input type="email">` | |
| `Field::password("pass")` | `<input type="password">` | Value not pre-filled on edit |
| `Field::number("count")` | `<input type="number">` | Integer |
| `Field::float("price")` | `<input type="number">` | Float |
| `Field::boolean("active")` | `<input type="checkbox">` | |
| `Field::date("created_at")` | `<input type="date">` | |
| `Field::datetime("updated_at")` | `<input type="datetime-local">` | |
| `Field::select("status", options)` | `<select>` | `options: Vec<(value, label)>` |
| `Field::json("metadata")` | `<textarea>` | |
| `Field::custom("slug", widget)` | Custom HTML | Implement `Widget` trait |

### Field modifiers

```rust
Field::text("slug")
    .label("URL Slug")       // override display label
    .required()              // adds required attribute
    .readonly()              // renders as non-editable
    .hidden()                // omit from all views
    .list_only()             // show in list, hide in form
    .form_only()             // show in form, hide in list
    .help_text("Used in the URL")
```

## Custom actions

Custom actions appear as buttons on the list page (bulk) or edit page (single record).

```rust
use axum_admin::entity::{CustomAction, ActionTarget, ActionResult};

EntityAdmin::new::<()>("users")
    // ...
    .action(
        CustomAction::builder("ban", "Ban Selected")
            .target(ActionTarget::List)
            .confirm("Ban these users?")
            .handler(|ctx| Box::pin(async move {
                // ctx.ids contains the selected record IDs
                let count = ctx.ids.len();
                // ... do the work ...
                Ok(ActionResult::Success(format!("Banned {count} users")))
            })),
    )
    .action(
        CustomAction::builder("impersonate", "Login as User")
            .target(ActionTarget::Detail)
            .handler(|ctx| Box::pin(async move {
                let id = ctx.ids.first().and_then(|v| v.as_str()).unwrap_or("");
                Ok(ActionResult::Redirect(format!("/impersonate/{id}")))
            })),
    )
```

`ActionResult` variants:
- `Success(message)` — renders a green flash message via HTMX swap
- `Error(message)` — renders a red flash message
- `Redirect(url)` — HTTP 302 redirect

## Lifecycle hooks

```rust
EntityAdmin::new::<()>("users")
    .before_save(|data| {
        // Validate or transform data before create/update
        if data.get("email").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
            return Err(AdminError::ValidationError(
                HashMap::from([("email".into(), "Email is required".into())])
            ));
        }
        Ok(())
    })
    .after_delete(|id| {
        // Clean up related data after a record is deleted
        println!("Deleted user {id}");
        Ok(())
    })
```

## Custom authentication

Implement `AdminAuth` to integrate with your existing user system:

```rust
use axum_admin::auth::{AdminAuth, AdminUser};
use axum_admin::AdminError;
use async_trait::async_trait;

struct MyAuth;

#[async_trait]
impl AdminAuth for MyAuth {
    async fn authenticate(&self, username: &str, password: &str) -> Result<AdminUser, AdminError> {
        // Verify credentials against your database
        // Return AdminUser { username, session_id } on success
        todo!()
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<AdminUser>, AdminError> {
        // Look up a session by ID; return None if expired/invalid
        todo!()
    }
}

AdminApp::new()
    .auth(Box::new(MyAuth))
    // ...
```

## License

MIT
