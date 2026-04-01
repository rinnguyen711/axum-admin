# axum-admin

A modern admin dashboard framework for [Axum](https://github.com/tokio-rs/axum). Register your entities and get a full CRUD dashboard — search, filtering, pagination, bulk actions, custom actions, and built-in authentication — with zero frontend build step.

Inspired by Django Admin and Laravel Nova.

## Features

- **CRUD out of the box** — list, create, edit, delete for any entity
- **Server-side rendering** — MiniJinja templates, no JavaScript framework required
- **HTMX** — partial page swaps for search, pagination, and flash messages
- **Alpine.js** — embedded, no CDN or build step
- **Built-in auth** — session-based login with bcrypt passwords; swap in your own auth backend
- **Sidebar groups** — collapse related entities under named, expandable sections
- **Custom icons** — Font Awesome icons for the app logo and each entity
- **Bulk actions** — delete, CSV export, and custom bulk handlers
- **Custom actions** — per-record actions with confirmation dialogs and HTMX flash responses
- **Filters & search** — collapsible filter panel, full-text search, column sorting
- **Template customization** — override any built-in template or load from a directory on disk
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
use axum_admin::{AdminApp, EntityAdmin, Field, DefaultAdminAuth};

#[tokio::main]
async fn main() {
    let app = AdminApp::new()
        .title("My App")
        .auth(Box::new(
            DefaultAdminAuth::new().add_user("admin", "secret"),
        ))
        .register(
            EntityAdmin::new::<()>("posts")
                .label("Posts")
                .field(Field::number("id").list_only())
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

Open `http://localhost:3000/admin/` to see the dashboard. Login with the credentials above.

## Sidebar groups

Use `EntityGroupAdmin` to group related entities under a collapsible sidebar section. Both `EntityAdmin` and `EntityGroupAdmin` can be passed to `.register()`.

```rust
use axum_admin::{AdminApp, EntityAdmin, EntityGroupAdmin};

AdminApp::new()
    // ungrouped entity — appears flat in the sidebar
    .register(
        EntityAdmin::new::<()>("settings").adapter(Box::new(SettingsAdapter)),
    )
    // grouped entities — collapsed under "Blog" in the sidebar
    .register(
        EntityGroupAdmin::new("Blog")
            .register(
                EntityAdmin::new::<()>("categories")
                    .icon("fa-solid fa-folder")
                    .adapter(Box::new(CategoriesAdapter)),
            )
            .register(
                EntityAdmin::new::<()>("posts")
                    .icon("fa-solid fa-file-lines")
                    .adapter(Box::new(PostsAdapter)),
            )
    )
```

The active group auto-expands when any of its children is the current page.

## Custom icons

Set Font Awesome icon classes on the app logo and individual entities:

```rust
AdminApp::new()
    .icon("fa-solid fa-gauge")   // sidebar logo; default: fa-solid fa-bolt

EntityAdmin::new::<()>("users")
    .icon("fa-solid fa-users")   // sidebar + dashboard card; default: fa-solid fa-layer-group
```

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

Enable the `seaorm` feature and use `SeaOrmAdapter<E>` with `EntityAdmin::from_entity`:

```toml
[dependencies]
axum-admin = { version = "0.1", features = ["seaorm"] }
sea-orm = { version = "1", features = ["sqlx-postgres", "runtime-tokio-rustls"] }
```

```rust
use axum_admin::adapters::seaorm::SeaOrmAdapter;

EntityAdmin::from_entity::<post::Entity>("posts")
    .label("Posts")
    .list_display(vec!["id".into(), "title".into(), "status".into()])
    .search_fields(vec!["title".into(), "body".into()])
    .adapter(Box::new(SeaOrmAdapter::<post::Entity>::new(db.clone())))
```

`from_entity` auto-generates fields from the SeaORM column definitions, including enum `Select` fields and FK foreign key fields. You can override any field with `.field(...)`.

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
| `Field::foreign_key("category_id", "Category", adapter, "id", "name")` | `<select>` | Populated from another entity |
| `Field::json("metadata")` | `<textarea>` | |
| `Field::custom("slug", widget)` | Custom HTML | Implement `Widget` trait |

### Field modifiers

```rust
Field::text("slug")
    .label("URL Slug")        // override display label
    .required()               // marks field as required
    .readonly()               // rendered as non-editable in forms
    .hidden()                 // omit from all views
    .list_only()              // show in list, hide in form
    .form_only()              // show in form, hide in list
    .help_text("Used in the URL")
```

## Filters and search

List pages include a collapsible filter panel. Filters default to `list_display` columns; override with `filter_fields` or provide a custom `Field` per filter:

```rust
EntityAdmin::new::<()>("posts")
    .filter_fields(vec!["status".into(), "category_id".into()])
    // custom filter widget for a specific column:
    .filter(
        Field::select("status", vec![
            ("draft".into(), "Draft".into()),
            ("published".into(), "Published".into()),
        ])
    )
```

## Bulk actions

Bulk delete and bulk CSV export are enabled by default. Disable them or add your own:

```rust
EntityAdmin::new::<()>("posts")
    .bulk_delete(false)
    .bulk_export(false)
    .action(
        CustomAction::builder("publish", "Publish Selected")
            .target(ActionTarget::List)
            .confirm("Publish these posts?")
            .handler(|ctx| Box::pin(async move {
                // ctx.ids contains selected record IDs
                Ok(ActionResult::Success(format!("Published {} posts", ctx.ids.len())))
            }))
    )
```

## Custom actions

Single-record actions appear on the edit page:

```rust
CustomAction::builder("impersonate", "Login as User")
    .target(ActionTarget::Detail)
    .handler(|ctx| Box::pin(async move {
        let id = ctx.ids.first().and_then(|v| v.as_str()).unwrap_or("");
        Ok(ActionResult::Redirect(format!("/impersonate/{id}")))
    }))
```

`ActionResult` variants:
- `Success(message)` — green flash message via HTMX swap
- `Error(message)` — red flash message
- `Redirect(url)` — HTTP 302 redirect

## Lifecycle hooks

```rust
EntityAdmin::new::<()>("users")
    .before_save(|data| {
        if data.get("email").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
            return Err(AdminError::ValidationError(
                HashMap::from([("email".into(), "Email is required".into())])
            ));
        }
        Ok(())
    })
    .after_delete(|id| {
        println!("Deleted user {id}");
        Ok(())
    })
```

`before_save` errors are displayed inline on the form as field-level validation messages. Use `__all__` as the key for non-field errors.

## Template customization

Override any built-in template by loading a directory — any `.html` file whose name matches a built-in template takes precedence:

```rust
AdminApp::new()
    .template_dir("templates/admin")
```

Or pass a template string directly (highest precedence):

```rust
AdminApp::new()
    .template("home.html", include_str!("templates/my_home.html"))
```

Templates use [MiniJinja](https://docs.rs/minijinja) and support full `{% extends %}` / `{% block %}` inheritance:

```html
{% extends "layout.html" %}
{% block content %}
  <h1>Welcome to {{ admin_title }}</h1>
  {% for entity in entities %}
    <a href="/admin/{{ entity.name }}/">{{ entity.label }}</a>
  {% endfor %}
{% endblock %}
```

Built-in templates: `layout.html`, `home.html`, `list.html`, `list_table.html`, `form.html`, `login.html`, `flash.html`.

## Custom authentication

Implement `AdminAuth` to integrate with your existing user system:

```rust
use axum_admin::auth::{AdminAuth, AdminUser};
use async_trait::async_trait;

struct MyAuth;

#[async_trait]
impl AdminAuth for MyAuth {
    async fn authenticate(&self, username: &str, password: &str) -> Result<AdminUser, AdminError> {
        // verify credentials, return AdminUser { username, session_id }
        todo!()
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<AdminUser>, AdminError> {
        // look up a session; return None if expired or invalid
        todo!()
    }
}

AdminApp::new().auth(Box::new(MyAuth))
```

## License

MIT
