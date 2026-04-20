# Quick Start

This guide gets you from zero to a running admin dashboard in 5 steps.

## 1. Set the database URL

```sh
export DATABASE_URL=postgres://user:password@localhost:5432/myapp
```

## 2. Run migrations

axum-admin ships its own migration set (users, sessions, RBAC rules). Apply them at startup:

```rust
use sea_orm_migration::MigratorTrait;
use axum_admin::adapters::seaorm::migration::Migrator;

Migrator::up(&db, None).await?;
```

## 3. Create the admin user

```rust
use axum_admin::SeaOrmAdminAuth;

let auth = SeaOrmAdminAuth::new(db.clone()).await?;
auth.ensure_user("admin", "secret").await?;
```

`ensure_user` is idempotent — safe to call on every startup.

## 4. Register entities and mount the router

```rust
use axum_admin::{AdminApp, EntityAdmin, Field};
use axum_admin::adapters::seaorm::SeaOrmAdapter;

let app = AdminApp::new()
    .title("My App Admin")
    .auth(auth)
    .register(
        EntityAdmin::new("posts")
            .label("Posts")
            .adapter(SeaOrmAdapter::<post::Entity>::new(db.clone()))
            .field(Field::new("title").label("Title"))
            .field(Field::new("body").label("Body").textarea())
    )
    .into_router("/admin");

let router = axum::Router::new().nest("/", app);
```

## 5. Start the server

```rust
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
axum::serve(listener, router).await?;
```

Visit `http://localhost:3000/admin` and log in with the credentials from step 3.
