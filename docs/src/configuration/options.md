# Configuration Options

All options are set via builder methods on `AdminApp`.

## AdminApp builder

```rust
AdminApp::new()
    .title("My App")                    // browser tab title and sidebar header (default: "Admin")
    .icon("fa-solid fa-bolt")           // Font Awesome icon class for the sidebar header (default: "fa-solid fa-bolt")
    .prefix("/admin")                   // URL prefix for all admin routes (default: "/admin")
    .auth(auth_backend)                 // set AdminAuth implementation (Box<dyn AdminAuth>)
    .seaorm_auth(auth)                  // set SeaOrmAdminAuth (seaorm feature — wires auth + enforcer)
    .upload_limit(20 * 1024 * 1024)    // max multipart upload size in bytes (default: 10 MiB)
    .register(entity_or_group)          // register an EntityAdmin or EntityGroupAdmin
    .template(name, content)            // override a built-in template by name
    .template_dir(path)                 // load template overrides from a directory
```

### Method reference

| Method | Parameter | Default | Description |
|--------|-----------|---------|-------------|
| `title` | `&str` | `"Admin"` | Sets the browser tab title and sidebar header text |
| `icon` | `&str` | `"fa-solid fa-bolt"` | Font Awesome icon class shown in the sidebar header |
| `prefix` | `&str` | `"/admin"` | URL prefix for all admin routes |
| `auth` | `Box<dyn AdminAuth>` | — | Sets the authentication backend |
| `seaorm_auth` | `SeaOrmAdminAuth` | — | SeaORM feature: wires auth and Casbin RBAC enforcer together |
| `upload_limit` | `usize` (bytes) | `10 * 1024 * 1024` | Maximum multipart body size in bytes |
| `register` | `EntityAdmin` or `EntityGroupAdmin` | — | Registers an entity or group for display in the admin |
| `template` | `(&str, &str)` | — | Override a built-in template by name (e.g. `"layout.html"`) |
| `template_dir` | `impl Into<PathBuf>` | — | Load template overrides from a directory; later calls take precedence |

## Environment variables

| Variable | Description |
|----------|-------------|
| `DATABASE_URL` | PostgreSQL connection string, required when using the `seaorm` feature |

## Feature flags

| Flag | Description |
|------|-------------|
| `seaorm` | Enables SeaORM adapter (`SeaOrmAdapter`), auth (`SeaOrmAdminAuth`), and RBAC via Casbin |

## Mounting

`AdminApp::into_router()` is async and returns an `axum::Router`:

```rust
let admin_router = AdminApp::new()
    .title("My App")
    .auth(Box::new(my_auth))
    .register(my_entity)
    .into_router().await;

let app = axum::Router::new()
    .merge(admin_router);
```

> **Note:** `.auth()` (or `.seaorm_auth()` when using the `seaorm` feature) must be called before `into_router()`, or it will panic at startup.
