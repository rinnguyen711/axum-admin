# axum-admin

A modern admin dashboard framework for [Axum](https://github.com/tokio-rs/axum). Register your entities and get a full CRUD dashboard — search, filtering, pagination, bulk actions, custom actions, and built-in authentication — with zero frontend build step.

Inspired by Django Admin and Laravel Nova.

![axum-admin dashboard](docs/src/image.png)

## Features

- CRUD out of the box — list, create, edit, delete for any entity
- Server-side rendering via MiniJinja, no JS framework required
- HTMX + Alpine.js embedded, no CDN or build step
- Session-based auth with argon2; swap in your own backend
- RBAC via Casbin — per-entity permissions tied to user roles (SeaORM feature)
- Sidebar groups with collapsible sections and custom icons
- Filters, search, column sorting, pagination
- Bulk actions (delete, CSV export) and per-record custom actions
- Lifecycle hooks: `before_save`, `after_delete`
- Template override support
- ORM-agnostic via `DataAdapter` trait
- First-party SeaORM adapter behind the `seaorm` feature flag

## Quick start

### 1. Add the dependency

```sh
cargo add axum-admin --features seaorm
cargo add axum tokio --features tokio/full
```

### 2. Set the database URL

```sh
export DATABASE_URL=postgres://user:password@localhost:5432/myapp
```

### 3. Run migrations

axum-admin ships its own migration set (users, sessions, RBAC rules). Apply them by calling `Migrator::up` at startup — the example below shows how. Your app's own migrations can run in the same migrator or separately.

### 4. Create the admin user

Use `ensure_user` during app initialization to create a default admin account if it doesn't exist yet:

```rust
let auth = SeaOrmAdminAuth::new(db.clone()).await?;
auth.ensure_user("admin", "secret").await?;
```

`ensure_user` is idempotent — safe to call on every startup. Change the password via the Users UI after first login.

### 5. Register entities and start the server

```rust
use axum_admin::{AdminApp, EntityAdmin, EntityGroupAdmin, Field};
use axum_admin::adapters::seaorm::{SeaOrmAdapter};
use axum_admin::adapters::seaorm_auth::SeaOrmAdminAuth;

#[tokio::main]
async fn main() {
    let db = sea_orm::Database::connect(std::env::var("DATABASE_URL").unwrap()).await.unwrap();

    // run migrations
    migration::Migrator::up(&db, None).await.unwrap();

    // create default admin user
    let auth = SeaOrmAdminAuth::new(db.clone()).await.unwrap();
    auth.ensure_user("admin", "admin").await.unwrap();

    let router = AdminApp::new()
        .title("My App")
        .prefix("/admin")
        .seaorm_auth(auth)
        .register(
            EntityGroupAdmin::new("Content")
                .register(
                    EntityAdmin::from_entity::<post::Entity>("posts")
                        .label("Posts")
                        .search_fields(vec!["title".to_string()])
                        .adapter(Box::new(SeaOrmAdapter::<post::Entity>::new(db.clone()))),
                ),
        )
        .into_router()
        .await;

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Admin running at http://localhost:3000/admin");
    axum::serve(listener, router).await.unwrap();
}
```

### 6. Access the dashboard

```sh
cargo run
# open http://localhost:3000/admin
```

Change the default password under **Users** after first login.

---

See [examples/blog](examples/blog) for a full working example with categories, posts, tags, foreign keys, many-to-many relations, filters, and RBAC roles.

Full documentation at [rinnguyen711.github.io/axum-admin](https://rinnguyen711.github.io/axum-admin/).

## License

MIT
